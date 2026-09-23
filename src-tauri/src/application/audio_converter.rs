use std::{
    ffi::OsString,
    fs,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
    time::Duration,
};

use crate::{
    domain::job::{AudioFormat, ExecutionOutcome},
    platform,
};

pub const SUPPORTED_INPUTS: &[&str] = &["mp3", "wav", "flac", "aac", "m4a", "ogg"];

pub struct AudioConversion<'a> {
    pub ffmpeg_path: &'a Path,
    pub ffprobe_path: Option<&'a Path>,
    pub input_path: &'a Path,
    pub output_path: &'a Path,
    pub format: AudioFormat,
    pub job_id: &'a str,
}

pub fn validate_request(
    input_path: &Path,
    output_path: &Path,
    format: AudioFormat,
) -> Result<(), String> {
    if !input_path.is_absolute() || !output_path.is_absolute() {
        return Err("Input and output paths must be absolute".into());
    }
    if !input_path.is_file() {
        return Err("The selected input file does not exist".into());
    }
    let input_extension = extension(input_path).ok_or("The input file has no extension")?;
    if !is_supported_input(input_path) {
        return Err(format!("Unsupported audio input format: {input_extension}"));
    }
    if output_path.exists() {
        return Err("The output file already exists".into());
    }
    if extension(output_path).as_deref() != Some(format.extension()) {
        return Err(format!(
            "The output file extension must be .{}",
            format.extension()
        ));
    }
    let output_parent = output_path
        .parent()
        .ok_or("The output location has no parent directory")?;
    if !output_parent.is_dir() {
        return Err("The output directory does not exist".into());
    }
    if input_path == output_path {
        return Err("Input and output paths must be different".into());
    }
    Ok(())
}

pub fn is_supported_input(path: &Path) -> bool {
    extension(path).is_some_and(|value| SUPPORTED_INPUTS.contains(&value.as_str()))
}

pub fn convert<F>(
    request: AudioConversion<'_>,
    cancelled: Arc<AtomicBool>,
    mut report_progress: F,
) -> Result<ExecutionOutcome, String>
where
    F: FnMut(u8),
{
    validate_request(request.input_path, request.output_path, request.format)?;
    if !request.ffmpeg_path.is_file() {
        return Err("The bundled FFmpeg executable could not be found".into());
    }

    let staged_path = staged_output_path(request.output_path, request.job_id)?;
    let _ = fs::remove_file(&staged_path);
    let duration = request
        .ffprobe_path
        .and_then(|path| probe_duration(path, request.input_path));

    let mut command = platform::background_command(request.ffmpeg_path);
    command
        .args(build_arguments(
            request.input_path,
            &staged_path,
            request.format,
        ))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|error| format!("FFmpeg could not be started: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or("FFmpeg progress pipe unavailable")?;
    let stderr = child.stderr.take().ok_or("FFmpeg error pipe unavailable")?;
    let (progress_tx, progress_rx) = mpsc::channel();

    let progress_thread = thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let _ = progress_tx.send(line);
        }
    });
    let error_thread = thread::spawn(move || {
        let mut message = String::new();
        let _ = BufReader::new(stderr).read_to_string(&mut message);
        message
    });

    let status = loop {
        for line in progress_rx.try_iter() {
            if line == "progress=end" {
                report_progress(100);
            } else if let (Some(total), Some(value)) = (duration, line.strip_prefix("out_time_us="))
            {
                if let Ok(microseconds) = value.parse::<f64>() {
                    let percent =
                        ((microseconds / 1_000_000.0) / total * 100.0).clamp(0.0, 99.0) as u8;
                    report_progress(percent);
                }
            }
        }

        if cancelled.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => thread::sleep(Duration::from_millis(60)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = fs::remove_file(&staged_path);
                return Err(format!("FFmpeg process monitoring failed: {error}"));
            }
        }
    };

    let _ = progress_thread.join();
    let error_message = error_thread.join().unwrap_or_default();

    let Some(status) = status else {
        let _ = fs::remove_file(staged_path);
        return Ok(ExecutionOutcome::Cancelled);
    };
    if !status.success() {
        let _ = fs::remove_file(staged_path);
        let detail = error_message.trim();
        return Err(if detail.is_empty() {
            format!("FFmpeg exited with status {status}")
        } else {
            format!("FFmpeg conversion failed: {detail}")
        });
    }
    if fs::metadata(&staged_path).map_or(true, |metadata| metadata.len() == 0) {
        let _ = fs::remove_file(staged_path);
        return Err("FFmpeg did not produce a valid output file".into());
    }

    fs::rename(&staged_path, request.output_path)
        .map_err(|error| format!("Converted output could not be finalized: {error}"))?;
    report_progress(100);
    Ok(ExecutionOutcome::Completed)
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
}

fn staged_output_path(output_path: &Path, job_id: &str) -> Result<PathBuf, String> {
    let parent = output_path
        .parent()
        .ok_or("Output directory is unavailable")?;
    let stem = output_path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or("Output filename is invalid")?;
    let extension = output_path
        .extension()
        .and_then(|value| value.to_str())
        .ok_or("Output extension is invalid")?;
    Ok(parent.join(format!(".{stem}.{job_id}.part.{extension}")))
}

fn build_arguments(input: &Path, output: &Path, format: AudioFormat) -> Vec<OsString> {
    let codec = match format {
        AudioFormat::Mp3 => "libmp3lame",
        AudioFormat::Wav => "pcm_s16le",
        AudioFormat::Flac => "flac",
        AudioFormat::Aac | AudioFormat::M4a => "aac",
        AudioFormat::Ogg => "libvorbis",
    };

    [
        OsString::from("-hide_banner"),
        OsString::from("-loglevel"),
        OsString::from("error"),
        OsString::from("-nostdin"),
        OsString::from("-n"),
        OsString::from("-i"),
        input.as_os_str().to_owned(),
        OsString::from("-map_metadata"),
        OsString::from("0"),
        OsString::from("-vn"),
        OsString::from("-c:a"),
        OsString::from(codec),
        OsString::from("-progress"),
        OsString::from("pipe:1"),
        OsString::from("-nostats"),
        output.as_os_str().to_owned(),
    ]
    .into()
}

fn probe_duration(ffprobe: &Path, input: &Path) -> Option<f64> {
    let output = platform::background_command(ffprobe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(input)
        .stdin(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform;
    use std::sync::atomic::AtomicBool;
    use tempfile::tempdir;

    #[test]
    fn codec_arguments_are_owned_and_do_not_use_a_shell() {
        let args = build_arguments(
            Path::new("C:/audio/input.wav"),
            Path::new("C:/audio/output.mp3"),
            AudioFormat::Mp3,
        );
        let rendered: Vec<_> = args
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect();

        assert!(rendered
            .windows(2)
            .any(|pair| pair == ["-c:a", "libmp3lame"]));
        assert_eq!(
            rendered.last().map(String::as_str),
            Some("C:/audio/output.mp3")
        );
    }

    #[test]
    fn m4a_output_uses_the_aac_codec_and_m4a_container_extension() {
        let args = build_arguments(
            Path::new("C:/audio/input.wav"),
            Path::new("C:/audio/output.m4a"),
            AudioFormat::M4a,
        );
        let rendered = args
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert!(rendered.windows(2).any(|pair| pair == ["-c:a", "aac"]));
        assert_eq!(
            rendered.last().map(String::as_str),
            Some("C:/audio/output.m4a")
        );
    }

    #[test]
    fn validation_rejects_existing_outputs() {
        let directory = tempdir().unwrap();
        let input = directory.path().join("input.wav");
        let output = directory.path().join("output.mp3");
        fs::write(&input, b"input").unwrap();
        fs::write(&output, b"existing").unwrap();

        let error = validate_request(&input, &output, AudioFormat::Mp3).unwrap_err();
        assert!(error.contains("already exists"));
    }

    #[test]
    fn staging_keeps_the_real_output_extension() {
        let output = Path::new("C:/audio/output.ogg");
        assert_eq!(
            staged_output_path(output, "42").unwrap(),
            PathBuf::from("C:/audio/.output.42.part.ogg")
        );
    }

    #[test]
    #[ignore = "runs the bundled FFmpeg binary"]
    fn bundled_engine_converts_every_mvp_audio_format() {
        let directory = tempdir().unwrap();
        let ffmpeg = platform::find_ffmpeg(None).expect("FFmpeg is required for this smoke test");
        let ffprobe = platform::find_ffprobe(None);
        let input = directory.path().join("source.wav");
        let generated = platform::background_command(&ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i"])
            .arg("sine=frequency=440:duration=0.25")
            .args(["-c:a", "pcm_s16le"])
            .arg(&input)
            .status()
            .unwrap();
        assert!(generated.success());

        for (index, format) in [
            AudioFormat::Mp3,
            AudioFormat::Wav,
            AudioFormat::Flac,
            AudioFormat::Aac,
            AudioFormat::M4a,
            AudioFormat::Ogg,
        ]
        .into_iter()
        .enumerate()
        {
            let output = directory
                .path()
                .join(format!("output-{}.{}", index, format.extension()));
            let outcome = convert(
                AudioConversion {
                    ffmpeg_path: &ffmpeg,
                    ffprobe_path: ffprobe.as_deref(),
                    input_path: &input,
                    output_path: &output,
                    format,
                    job_id: &index.to_string(),
                },
                Arc::new(AtomicBool::new(false)),
                |_| {},
            )
            .unwrap();

            assert!(matches!(outcome, ExecutionOutcome::Completed));
            assert!(fs::metadata(output).unwrap().len() > 0);
        }
    }
}

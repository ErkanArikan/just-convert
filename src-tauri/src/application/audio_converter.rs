use std::{
    ffi::OsString,
    fs,
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};

use crate::{
    application::ffmpeg,
    domain::job::{AudioFormat, ExecutionOutcome},
};

pub const SUPPORTED_INPUTS: &[&str] = &[
    "mp3", "wav", "flac", "aac", "m4a", "ogg", "mp4", "mov", "mkv", "webm", "avi", "m4v",
];

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

    let staged_path = ffmpeg::staged_output_path(request.output_path, request.job_id)?;
    let _ = fs::remove_file(&staged_path);
    let duration = request
        .ffprobe_path
        .and_then(|path| ffmpeg::probe_duration(path, request.input_path));

    let libvorbis =
        request.format == AudioFormat::Ogg && ffmpeg::has_encoder(request.ffmpeg_path, "libvorbis");
    let outcome = ffmpeg::run(
        request.ffmpeg_path,
        build_arguments(request.input_path, &staged_path, request.format, libvorbis),
        duration,
        &cancelled,
        &mut report_progress,
    );
    match outcome {
        Ok(ExecutionOutcome::Completed) => {}
        other => {
            let _ = fs::remove_file(&staged_path);
            return other;
        }
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

// Homebrew and some distro FFmpeg builds lack libvorbis; the built-in encoder is experimental and stereo-only.
fn codec_arguments(format: AudioFormat, libvorbis: bool) -> &'static [&'static str] {
    match format {
        AudioFormat::Mp3 => &["-c:a", "libmp3lame"],
        AudioFormat::Wav => &["-c:a", "pcm_s16le"],
        AudioFormat::Flac => &["-c:a", "flac"],
        AudioFormat::Aac | AudioFormat::M4a => &["-c:a", "aac"],
        AudioFormat::Ogg if libvorbis => &["-c:a", "libvorbis"],
        AudioFormat::Ogg => &["-c:a", "vorbis", "-strict", "experimental", "-ac", "2"],
    }
}

fn build_arguments(
    input: &Path,
    output: &Path,
    format: AudioFormat,
    libvorbis: bool,
) -> Vec<OsString> {
    let mut arguments = ffmpeg::base_arguments(input);
    arguments.extend(["-map_metadata", "0", "-vn"].map(OsString::from));
    arguments.extend(
        codec_arguments(format, libvorbis)
            .iter()
            .map(OsString::from),
    );
    arguments.extend(ffmpeg::progress_arguments());
    arguments.push(output.as_os_str().to_owned());
    arguments
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
            false,
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
            false,
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
    fn ogg_falls_back_to_the_builtin_vorbis_encoder_without_libvorbis() {
        assert_eq!(
            codec_arguments(AudioFormat::Ogg, true),
            ["-c:a", "libvorbis"]
        );
        assert_eq!(
            codec_arguments(AudioFormat::Ogg, false),
            ["-c:a", "vorbis", "-strict", "experimental", "-ac", "2"]
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

use std::{
    ffi::OsString,
    fs,
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};

use crate::{
    application::{
        audio_converter::{self, AudioConversion},
        ffmpeg,
    },
    domain::job::{AudioFormat, ExecutionOutcome, TtsSpeed, TtsVoice},
};

// The runtime takes the text as one command-line argument, which Windows limits to about 32,000 characters.
pub const MAX_TEXT_CHARS: usize = 20_000;

pub struct Synthesis<'a> {
    pub runtime_path: &'a Path,
    pub voices_directory: &'a Path,
    pub ffmpeg_path: Option<&'a Path>,
    pub ffprobe_path: Option<&'a Path>,
    pub text: &'a str,
    pub output_path: &'a Path,
    pub format: AudioFormat,
    pub voice: TtsVoice,
    pub speed: TtsSpeed,
    pub job_id: &'a str,
}

pub fn validate_request(text: &str, output_path: &Path, format: AudioFormat) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err("There is no text to read aloud".into());
    }
    let length = text.chars().count();
    if length > MAX_TEXT_CHARS {
        return Err(format!(
            "The text is too long ({length} characters); the limit is {MAX_TEXT_CHARS}. Split it into parts"
        ));
    }
    if !output_path.is_absolute() {
        return Err("The output path must be absolute".into());
    }
    if output_path.exists() {
        return Err("The output file already exists".into());
    }
    let extension = output_path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);
    if extension.as_deref() != Some(format.extension()) {
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
    Ok(())
}

pub fn convert<F>(
    request: Synthesis<'_>,
    cancelled: Arc<AtomicBool>,
    mut report_progress: F,
) -> Result<ExecutionOutcome, String>
where
    F: FnMut(u8),
{
    validate_request(request.text, request.output_path, request.format)?;
    if !request.runtime_path.is_file() {
        return Err("The bundled text-to-speech engine could not be found".into());
    }
    let voice_directory = request
        .voices_directory
        .join("voices")
        .join(request.voice.directory());
    let model = voice_directory.join("model.onnx");
    let tokens = voice_directory.join("tokens.txt");
    let data = request.voices_directory.join("espeak-ng-data");
    if !model.is_file() || !tokens.is_file() || !data.is_dir() {
        return Err("The bundled voice could not be found".into());
    }
    let needs_encoder = request.format != AudioFormat::Wav;
    let ffmpeg_path = match (needs_encoder, request.ffmpeg_path) {
        (true, Some(path)) => Some(path),
        (true, None) => return Err("The bundled FFmpeg executable is unavailable".into()),
        (false, _) => None,
    };

    let wav_path = if needs_encoder {
        let parent = request
            .output_path
            .parent()
            .ok_or("The output location has no parent directory")?;
        parent.join(format!(".speech.{}.part.wav", request.job_id))
    } else {
        ffmpeg::staged_output_path(request.output_path, request.job_id)?
    };
    let _ = fs::remove_file(&wav_path);

    let synthesis_share = if needs_encoder { 85.0 } else { 98.0 };
    let outcome = ffmpeg::supervise(
        request.runtime_path,
        arguments(&model, &tokens, &data, &wav_path, &request),
        "The text-to-speech engine",
        "synthesis",
        &cancelled,
        |line| {
            if let Some(progress) = line
                .split("progress=")
                .nth(1)
                .and_then(|value| value.trim().parse::<f64>().ok())
            {
                report_progress((progress.clamp(0.0, 1.0) * synthesis_share) as u8);
            }
        },
    );
    match outcome {
        Ok(ExecutionOutcome::Completed) => {}
        other => {
            let _ = fs::remove_file(&wav_path);
            return other;
        }
    }
    // A WAV header alone is 44 bytes.
    if fs::metadata(&wav_path).map_or(true, |metadata| metadata.len() <= 44) {
        let _ = fs::remove_file(&wav_path);
        return Err("The text-to-speech engine did not produce any audio".into());
    }

    let Some(ffmpeg_path) = ffmpeg_path else {
        fs::rename(&wav_path, request.output_path)
            .map_err(|error| format!("The audio could not be finalized: {error}"))?;
        report_progress(100);
        return Ok(ExecutionOutcome::Completed);
    };
    let encoded = audio_converter::convert(
        AudioConversion {
            ffmpeg_path,
            ffprobe_path: request.ffprobe_path,
            input_path: &wav_path,
            output_path: request.output_path,
            format: request.format,
            job_id: request.job_id,
        },
        cancelled,
        |progress| report_progress(85 + (u16::from(progress) * 15 / 100) as u8),
    );
    let _ = fs::remove_file(&wav_path);
    encoded
}

fn arguments(
    model: &Path,
    tokens: &Path,
    data: &Path,
    output: &Path,
    request: &Synthesis<'_>,
) -> Vec<OsString> {
    let threads = std::thread::available_parallelism().map_or(2, |count| count.get().min(4));
    let flag = |name: &str, value: &Path| {
        let mut argument = OsString::from(format!("--{name}="));
        argument.push(value.as_os_str());
        argument
    };
    let mut text = request.text.trim().replace(['\r', '\0'], "");
    // A leading dash would be read as an option.
    if text.starts_with('-') {
        text.insert(0, ' ');
    }
    vec![
        flag("vits-model", model),
        flag("vits-tokens", tokens),
        flag("vits-data-dir", data),
        OsString::from(format!(
            "--vits-length-scale={}",
            request.speed.length_scale()
        )),
        OsString::from(format!("--num-threads={threads}")),
        flag("output-filename", output),
        OsString::from(text),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn synthesis<'a>(
        text: &'a str,
        output: &'a Path,
        paths: &'a platform::TtsPaths,
        voice: TtsVoice,
        format: AudioFormat,
    ) -> Synthesis<'a> {
        Synthesis {
            runtime_path: &paths.executable,
            voices_directory: &paths.voices_directory,
            ffmpeg_path: None,
            ffprobe_path: None,
            text,
            output_path: output,
            format,
            voice,
            speed: TtsSpeed::Normal,
            job_id: "1",
        }
    }

    #[test]
    fn validation_rejects_bad_requests() {
        let directory = tempdir().unwrap();
        let output = directory.path().join("speech.mp3");
        let existing = directory.path().join("existing.mp3");
        fs::write(&existing, b"x").unwrap();

        assert!(validate_request("   ", &output, AudioFormat::Mp3)
            .unwrap_err()
            .contains("no text"));
        assert!(validate_request("hi", &existing, AudioFormat::Mp3)
            .unwrap_err()
            .contains("already exists"));
        assert!(validate_request("hi", &output, AudioFormat::Wav)
            .unwrap_err()
            .contains("must be .wav"));
        assert!(
            validate_request(&"a".repeat(MAX_TEXT_CHARS + 1), &output, AudioFormat::Mp3)
                .unwrap_err()
                .contains("too long")
        );
        assert!(validate_request("hello", &output, AudioFormat::Mp3).is_ok());
    }

    #[test]
    fn arguments_keep_the_text_out_of_option_position() {
        let paths = platform::TtsPaths {
            executable: PathBuf::from("/e/tts"),
            voices_directory: PathBuf::from("/v"),
        };
        let output = Path::new("/o/out.wav");
        let request = synthesis(
            "-5 degrees today",
            output,
            &paths,
            TtsVoice::English,
            AudioFormat::Wav,
        );
        let rendered: Vec<String> = arguments(
            Path::new("/v/voices/en/model.onnx"),
            Path::new("/v/voices/en/tokens.txt"),
            Path::new("/v/espeak-ng-data"),
            output,
            &request,
        )
        .iter()
        .map(|value| value.to_string_lossy().into_owned())
        .collect();

        assert_eq!(rendered[0], "--vits-model=/v/voices/en/model.onnx");
        assert!(rendered.contains(&"--vits-length-scale=1.0".to_string()));
        assert_eq!(rendered.last().unwrap(), " -5 degrees today");
    }

    fn paths() -> platform::TtsPaths {
        platform::find_tts(None).expect("run `pnpm fetch:engines` first")
    }

    fn wav_duration(path: &Path) -> f64 {
        let bytes = fs::read(path).unwrap();
        assert_eq!(&bytes[..4], b"RIFF");
        let sample_rate = u32::from_le_bytes(bytes[24..28].try_into().unwrap());
        let byte_rate = u32::from_le_bytes(bytes[28..32].try_into().unwrap());
        assert!(sample_rate >= 16_000);
        (bytes.len() - 44) as f64 / f64::from(byte_rate)
    }

    #[test]
    #[ignore = "needs the fetched text-to-speech engine and voices"]
    fn speaks_english_and_turkish_to_wav() {
        let directory = tempdir().unwrap();
        let paths = paths();
        for (voice, text, name) in [
            (
                TtsVoice::English,
                "Hello from Just Convert. This voice runs on this computer.",
                "en.wav",
            ),
            (
                TtsVoice::Turkish,
                "Merhaba, bu ses bu bilgisayarda çalışır. Çocuğun güzel şehri İstanbul.",
                "tr.wav",
            ),
        ] {
            let output = directory.path().join(name);
            let mut last = 0;
            let outcome = convert(
                synthesis(text, &output, &paths, voice, AudioFormat::Wav),
                Arc::new(AtomicBool::new(false)),
                |progress| last = progress,
            )
            .unwrap();
            assert!(matches!(outcome, ExecutionOutcome::Completed));
            assert_eq!(last, 100);
            assert!(wav_duration(&output) > 2.0, "{name}");
        }
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
    }

    #[test]
    #[ignore = "needs the fetched text-to-speech engine, voices, and FFmpeg"]
    fn encodes_speech_to_mp3_and_removes_the_temporary_wav() {
        let directory = tempdir().unwrap();
        let paths = paths();
        let ffmpeg = platform::find_ffmpeg(None).expect("FFmpeg is required");
        let output = directory.path().join("speech.mp3");
        let mut request = synthesis(
            "Testing the encoded output.",
            &output,
            &paths,
            TtsVoice::English,
            AudioFormat::Mp3,
        );
        request.ffmpeg_path = Some(&ffmpeg);

        let outcome = convert(request, Arc::new(AtomicBool::new(false)), |_| {}).unwrap();

        assert!(matches!(outcome, ExecutionOutcome::Completed));
        assert!(fs::metadata(&output).unwrap().len() > 1000);
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    #[ignore = "needs the fetched text-to-speech engine and voices"]
    fn cancellation_leaves_no_files_behind() {
        let directory = tempdir().unwrap();
        let paths = paths();
        let output = directory.path().join("speech.wav");
        let outcome = convert(
            synthesis(
                &"This is a long sentence that keeps going. ".repeat(40),
                &output,
                &paths,
                TtsVoice::English,
                AudioFormat::Wav,
            ),
            Arc::new(AtomicBool::new(true)),
            |_| {},
        )
        .unwrap();

        assert!(matches!(outcome, ExecutionOutcome::Cancelled));
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}

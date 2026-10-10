use std::{
    ffi::c_void,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc, Once,
    },
    thread,
    time::Duration,
};

use serde_json::json;
use sha2::{Digest, Sha256};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::{
    application::audio_converter,
    domain::job::{ExecutionOutcome, SpeechModel, TranscriptFormat},
    platform,
};

pub const STANDARD_MODEL_FILE: &str = "ggml-base-q5_1.bin";

pub struct DownloadableModel {
    pub file_name: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
    pub size: u64,
}

pub const HIGH_ACCURACY_MODEL: DownloadableModel = DownloadableModel {
    file_name: "ggml-large-v3-turbo-q5_0.bin",
    url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-large-v3-turbo-q5_0.bin",
    sha256: "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2",
    size: 574_041_195,
};

const SAMPLE_RATE: u32 = 16_000;

pub struct Transcription<'a> {
    pub ffmpeg_path: &'a Path,
    pub model_path: &'a Path,
    pub input_path: &'a Path,
    pub output_path: &'a Path,
    pub format: TranscriptFormat,
    pub language: Option<&'a str>,
    pub job_id: &'a str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
}

pub fn is_supported_input(path: &Path) -> bool {
    audio_converter::is_supported_input(path)
}

pub fn validate_language(language: Option<&str>) -> Result<(), String> {
    match language {
        None | Some("auto") => Ok(()),
        Some(code) if whisper_rs::get_lang_id(code).is_some() => Ok(()),
        Some(code) => Err(format!("Unsupported transcription language: {code}")),
    }
}

pub fn validate_request(
    input_path: &Path,
    output_path: &Path,
    format: TranscriptFormat,
    language: Option<&str>,
) -> Result<(), String> {
    if !input_path.is_absolute() || !output_path.is_absolute() {
        return Err("Input and output paths must be absolute".into());
    }
    if !input_path.is_file() {
        return Err("The selected input file does not exist".into());
    }
    if !is_supported_input(input_path) {
        return Err("Transcription needs an audio or video file".into());
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
    if output_path.parent().is_none_or(|parent| !parent.is_dir()) {
        return Err("The output directory does not exist".into());
    }
    validate_language(language)
}

/// Returns the model to use, downloading the high-accuracy model on first use.
/// `None` means the download was cancelled.
pub fn ensure_model<F>(
    model: SpeechModel,
    bundled_standard_model: Option<&Path>,
    models_directory: &Path,
    cancelled: &AtomicBool,
    mut report_progress: F,
) -> Result<Option<PathBuf>, String>
where
    F: FnMut(u8),
{
    match model {
        SpeechModel::Standard => bundled_standard_model
            .filter(|path| path.is_file())
            .map(|path| Some(path.to_path_buf()))
            .ok_or_else(|| "The bundled speech model is unavailable".into()),
        SpeechModel::HighAccuracy => {
            let destination = models_directory.join(HIGH_ACCURACY_MODEL.file_name);
            let verified = models_directory.join(format!(
                "{}.{}.verified",
                HIGH_ACCURACY_MODEL.file_name, HIGH_ACCURACY_MODEL.sha256
            ));
            if destination.is_file() && verified.is_file() {
                return Ok(Some(destination));
            }
            fs::create_dir_all(models_directory)
                .map_err(|error| format!("Model directory could not be created: {error}"))?;
            if !download(
                &HIGH_ACCURACY_MODEL,
                &destination,
                cancelled,
                &mut report_progress,
            )? {
                return Ok(None);
            }
            fs::write(&verified, b"")
                .map_err(|error| format!("Model verification could not be recorded: {error}"))?;
            Ok(Some(destination))
        }
    }
}

fn download<F>(
    model: &DownloadableModel,
    destination: &Path,
    cancelled: &AtomicBool,
    report_progress: &mut F,
) -> Result<bool, String>
where
    F: FnMut(u8),
{
    let partial = destination.with_extension("part");
    let response = ureq::get(model.url)
        .call()
        .map_err(|error| format!("The speech model could not be downloaded: {error}"))?;
    let mut reader = response.into_body().into_reader();
    let mut file = fs::File::create(&partial)
        .map_err(|error| format!("The speech model could not be saved: {error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 1 << 16];
    let mut written: u64 = 0;
    loop {
        if cancelled.load(Ordering::Relaxed) {
            drop(file);
            let _ = fs::remove_file(&partial);
            return Ok(false);
        }
        let read = reader
            .read(&mut buffer)
            .map_err(|error| format!("The speech model download failed: {error}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        file.write_all(&buffer[..read])
            .map_err(|error| format!("The speech model could not be saved: {error}"))?;
        written += read as u64;
        report_progress(((written * 100) / model.size).min(100) as u8);
    }
    drop(file);
    let actual = format!("{:x}", hasher.finalize());
    if actual != model.sha256 {
        let _ = fs::remove_file(&partial);
        return Err("The downloaded speech model failed its checksum".into());
    }
    fs::rename(&partial, destination)
        .map_err(|error| format!("The speech model could not be saved: {error}"))?;
    Ok(true)
}

pub fn convert<F>(
    request: Transcription<'_>,
    cancelled: Arc<AtomicBool>,
    mut report_progress: F,
) -> Result<ExecutionOutcome, String>
where
    F: FnMut(u8),
{
    validate_request(
        request.input_path,
        request.output_path,
        request.format,
        request.language,
    )?;
    if !request.ffmpeg_path.is_file() {
        return Err("The bundled FFmpeg executable could not be found".into());
    }

    let Some(samples) = decode(request.ffmpeg_path, request.input_path, &cancelled)? else {
        return Ok(ExecutionOutcome::Cancelled);
    };
    report_progress(10);

    let Some((segments, language)) = transcribe(
        request.model_path,
        &samples,
        request.language,
        &cancelled,
        |progress| report_progress(10 + progress * 85 / 100),
    )?
    else {
        return Ok(ExecutionOutcome::Cancelled);
    };
    if segments.is_empty() {
        return Err("No speech was detected in the input".into());
    }

    let staged = staged_output_path(request.output_path, request.job_id)?;
    fs::write(&staged, render(&segments, request.format, &language))
        .map_err(|error| format!("The transcript could not be written: {error}"))?;
    if request.output_path.exists() {
        let _ = fs::remove_file(&staged);
        return Err("The output file already exists".into());
    }
    fs::rename(&staged, request.output_path)
        .map_err(|error| format!("The transcript could not be finalized: {error}"))?;
    report_progress(100);
    Ok(ExecutionOutcome::Completed)
}

fn decode(
    ffmpeg_path: &Path,
    input_path: &Path,
    cancelled: &AtomicBool,
) -> Result<Option<Vec<f32>>, String> {
    let mut child = platform::background_command(ffmpeg_path)
        .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-i"])
        .arg(input_path)
        .args(["-vn", "-ac", "1", "-ar"])
        .arg(SAMPLE_RATE.to_string())
        .args(["-f", "f32le", "-"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("FFmpeg could not be started: {error}"))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or("FFmpeg output pipe unavailable")?;
    let mut stderr = child.stderr.take().ok_or("FFmpeg error pipe unavailable")?;
    let reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.read_to_end(&mut bytes);
        bytes
    });
    let errors = thread::spawn(move || {
        let mut message = String::new();
        let _ = stderr.read_to_string(&mut message);
        message
    });

    let status = loop {
        if cancelled.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            return Ok(None);
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(error) => {
                let _ = child.kill();
                return Err(format!("FFmpeg process monitoring failed: {error}"));
            }
        }
    };
    let bytes = reader.join().unwrap_or_default();
    let message = errors.join().unwrap_or_default();
    if !status.success() {
        let detail = message.trim();
        return Err(if detail.is_empty() {
            format!("FFmpeg exited with status {status}")
        } else {
            format!("Audio could not be decoded: {detail}")
        });
    }
    let samples: Vec<f32> = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| f32::from_le_bytes(*chunk))
        .collect();
    if samples.is_empty() {
        return Err("The input contains no audio".into());
    }
    Ok(Some(samples))
}

type TranscriptionResult = (Vec<Segment>, String);

fn transcribe<F>(
    model_path: &Path,
    samples: &[f32],
    language: Option<&str>,
    cancelled: &Arc<AtomicBool>,
    mut report_progress: F,
) -> Result<Option<TranscriptionResult>, String>
where
    F: FnMut(u8),
{
    static SILENCE_WHISPER_LOGS: Once = Once::new();
    SILENCE_WHISPER_LOGS.call_once(whisper_rs::install_logging_hooks);

    let model = model_path
        .to_str()
        .ok_or("The speech model path is not valid UTF-8")?;
    let context = WhisperContext::new_with_params(model, WhisperContextParameters::default())
        .map_err(|error| format!("The speech model could not be loaded: {error}"))?;
    let progress = Arc::new(AtomicU8::new(0));
    let threads = thread::available_parallelism()
        .map(|count| count.get().min(8))
        .unwrap_or(4) as i32;

    let result = thread::scope(|scope| {
        let worker = scope.spawn(|| -> Result<TranscriptionResult, String> {
            let mut state = context
                .create_state()
                .map_err(|error| format!("The speech model could not be started: {error}"))?;
            let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
            params.set_language(Some(language.unwrap_or("auto")));
            params.set_n_threads(threads);
            params.set_print_special(false);
            params.set_print_progress(false);
            params.set_print_realtime(false);
            params.set_print_timestamps(false);
            let progress_sink = Arc::clone(&progress);
            params.set_progress_callback_safe(move |value: i32| {
                progress_sink.store(value.clamp(0, 100) as u8, Ordering::Relaxed);
            });
            // whisper-rs 0.16's set_abort_callback_safe casts its user data to the wrong type,
            // so the cancellation flag is passed through the raw callback instead.
            // SAFETY: `cancelled` outlives `state.full`, the only caller of the callback.
            unsafe {
                params.set_abort_callback(Some(abort_requested));
                params.set_abort_callback_user_data(Arc::as_ptr(cancelled) as *mut c_void);
            }

            state
                .full(params, samples)
                .map_err(|error| format!("Transcription failed: {error}"))?;
            let detected = whisper_rs::get_lang_str(state.full_lang_id_from_state())
                .unwrap_or("unknown")
                .to_string();
            let mut segments = Vec::new();
            for segment in state.as_iter() {
                let text = segment
                    .to_str_lossy()
                    .map_err(|error| format!("Transcript text could not be read: {error}"))?
                    .trim()
                    .to_string();
                if !text.is_empty() {
                    segments.push(Segment {
                        start_ms: segment.start_timestamp() * 10,
                        end_ms: segment.end_timestamp() * 10,
                        text,
                    });
                }
            }
            Ok((segments, detected))
        });
        while !worker.is_finished() {
            report_progress(progress.load(Ordering::Relaxed));
            thread::sleep(Duration::from_millis(200));
        }
        worker
            .join()
            .map_err(|_| "The transcription worker stopped unexpectedly".to_string())
    })?;

    if cancelled.load(Ordering::Relaxed) {
        return Ok(None);
    }
    result.map(Some)
}

unsafe extern "C" fn abort_requested(user_data: *mut c_void) -> bool {
    // SAFETY: the user data is the job's cancellation flag, set in `transcribe`.
    unsafe { (*(user_data as *const AtomicBool)).load(Ordering::Relaxed) }
}

pub fn render(segments: &[Segment], format: TranscriptFormat, language: &str) -> String {
    match format {
        TranscriptFormat::Txt => {
            let mut text = segments
                .iter()
                .map(|segment| segment.text.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            text.push('\n');
            text
        }
        TranscriptFormat::Srt => segments
            .iter()
            .enumerate()
            .map(|(index, segment)| {
                format!(
                    "{}\n{} --> {}\n{}\n\n",
                    index + 1,
                    timestamp(segment.start_ms, ','),
                    timestamp(segment.end_ms, ','),
                    segment.text
                )
            })
            .collect(),
        TranscriptFormat::Vtt => {
            let mut text = String::from("WEBVTT\n\n");
            for segment in segments {
                text.push_str(&format!(
                    "{} --> {}\n{}\n\n",
                    timestamp(segment.start_ms, '.'),
                    timestamp(segment.end_ms, '.'),
                    segment.text
                ));
            }
            text
        }
        TranscriptFormat::Json => {
            let value = json!({
                "language": language,
                "text": segments.iter().map(|segment| segment.text.as_str()).collect::<Vec<_>>().join(" "),
                "segments": segments.iter().map(|segment| json!({
                    "start": segment.start_ms as f64 / 1000.0,
                    "end": segment.end_ms as f64 / 1000.0,
                    "text": segment.text,
                })).collect::<Vec<_>>(),
            });
            format!(
                "{}\n",
                serde_json::to_string_pretty(&value).unwrap_or_default()
            )
        }
    }
}

fn timestamp(milliseconds: i64, separator: char) -> String {
    let milliseconds = milliseconds.max(0);
    format!(
        "{:02}:{:02}:{:02}{separator}{:03}",
        milliseconds / 3_600_000,
        milliseconds / 60_000 % 60,
        milliseconds / 1000 % 60,
        milliseconds % 1000
    )
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
    let name = output_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("Output filename is invalid")?;
    Ok(parent.join(format!(".{name}.{job_id}.part")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample() -> Vec<Segment> {
        vec![
            Segment {
                start_ms: 0,
                end_ms: 2_500,
                text: "Hello there.".into(),
            },
            Segment {
                start_ms: 3_723_456,
                end_ms: 3_725_000,
                text: "Goodbye.".into(),
            },
        ]
    }

    #[test]
    fn srt_uses_numbered_cues_and_comma_milliseconds() {
        assert_eq!(
            render(&sample(), TranscriptFormat::Srt, "en"),
            "1\n00:00:00,000 --> 00:00:02,500\nHello there.\n\n2\n01:02:03,456 --> 01:02:05,000\nGoodbye.\n\n"
        );
    }

    #[test]
    fn vtt_has_a_header_and_dot_milliseconds() {
        let vtt = render(&sample(), TranscriptFormat::Vtt, "en");
        assert!(vtt.starts_with("WEBVTT\n\n00:00:00.000 --> 00:00:02.500\nHello there.\n"));
    }

    #[test]
    fn text_and_json_keep_every_segment() {
        assert_eq!(
            render(&sample(), TranscriptFormat::Txt, "en"),
            "Hello there.\nGoodbye.\n"
        );
        let json: serde_json::Value =
            serde_json::from_str(&render(&sample(), TranscriptFormat::Json, "tr")).unwrap();
        assert_eq!(json["language"], "tr");
        assert_eq!(json["segments"][1]["start"], 3723.456);
    }

    #[test]
    fn validation_requires_matching_extension_and_known_language() {
        let directory = tempdir().unwrap();
        let input = directory.path().join("talk.mp4");
        fs::write(&input, b"").unwrap();
        let output = directory.path().join("talk.txt");

        assert!(validate_request(&input, &output, TranscriptFormat::Txt, None).is_ok());
        assert!(
            validate_request(&input, &output, TranscriptFormat::Srt, None)
                .unwrap_err()
                .contains(".srt")
        );
        assert!(validate_request(&input, &output, TranscriptFormat::Txt, Some("tr")).is_ok());
        assert!(
            validate_request(&input, &output, TranscriptFormat::Txt, Some("xx"))
                .unwrap_err()
                .contains("language")
        );
    }

    #[test]
    fn standard_model_requires_the_bundled_file() {
        let directory = tempdir().unwrap();
        let missing = directory.path().join(STANDARD_MODEL_FILE);
        let result = ensure_model(
            SpeechModel::Standard,
            Some(&missing),
            directory.path(),
            &AtomicBool::new(false),
            |_| {},
        );
        assert!(result.unwrap_err().contains("bundled speech model"));
    }

    #[test]
    #[ignore = "runs the bundled FFmpeg binary and Whisper model"]
    fn bundled_model_transcribes_english_speech() {
        let ffmpeg = platform::find_ffmpeg(None).expect("FFmpeg is required");
        let model = platform::find_whisper_model(None).expect("the Whisper model is required");
        let directory = tempdir().unwrap();
        let input = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/speech-en.wav");
        let output = directory.path().join("speech.srt");
        let mut progress = Vec::new();

        let outcome = convert(
            Transcription {
                ffmpeg_path: &ffmpeg,
                model_path: &model,
                input_path: &input,
                output_path: &output,
                format: TranscriptFormat::Srt,
                language: None,
                job_id: "speech",
            },
            Arc::new(AtomicBool::new(false)),
            |value| progress.push(value),
        )
        .unwrap();

        assert!(matches!(outcome, ExecutionOutcome::Completed));
        let transcript = fs::read_to_string(output).unwrap().to_lowercase();
        assert!(transcript.contains("00:00:00,000 -->"), "{transcript}");
        assert!(transcript.contains("convert"), "{transcript}");
        assert_eq!(progress.last(), Some(&100));
    }

    #[test]
    #[ignore = "runs the bundled Whisper model"]
    fn cancellation_stops_whisper_without_output() {
        let model = platform::find_whisper_model(None).expect("the Whisper model is required");
        let samples = vec![0.0_f32; SAMPLE_RATE as usize * 5];

        let result = transcribe(
            &model,
            &samples,
            None,
            &Arc::new(AtomicBool::new(true)),
            |_| {},
        );

        assert_eq!(result, Ok(None));
    }
}

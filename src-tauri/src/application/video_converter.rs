use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc},
};

use crate::{
    application::ffmpeg,
    domain::job::{ExecutionOutcome, PdfImageFormat, VideoFormat, VideoQuality},
};

pub const SUPPORTED_INPUTS: &[&str] = &[
    "mp4", "mov", "mkv", "webm", "avi", "m4v", "mpg", "mpeg", "wmv", "flv", "3gp", "gif",
];
pub const MIN_HEIGHT: u32 = 144;
pub const MAX_HEIGHT: u32 = 4320;
pub const MAX_FRAME_INTERVAL_SECONDS: u32 = 3600;

pub struct VideoConversion<'a> {
    pub ffmpeg_path: &'a Path,
    pub ffprobe_path: Option<&'a Path>,
    pub input_path: &'a Path,
    pub output_path: &'a Path,
    pub format: VideoFormat,
    pub quality: VideoQuality,
    pub max_height: Option<u32>,
    pub job_id: &'a str,
}

pub struct FrameExtraction<'a> {
    pub ffmpeg_path: &'a Path,
    pub ffprobe_path: Option<&'a Path>,
    pub input_path: &'a Path,
    pub output_directory: &'a Path,
    pub format: PdfImageFormat,
    pub interval_seconds: u32,
    pub job_id: &'a str,
}

pub fn is_supported_input(path: &Path) -> bool {
    extension(path).is_some_and(|value| SUPPORTED_INPUTS.contains(&value.as_str()))
}

fn validate_input(input_path: &Path) -> Result<(), String> {
    if !input_path.is_absolute() {
        return Err("Input and output paths must be absolute".into());
    }
    if !input_path.is_file() {
        return Err("The selected input file does not exist".into());
    }
    let input_extension = extension(input_path).ok_or("The input file has no extension")?;
    if !is_supported_input(input_path) {
        return Err(format!("Unsupported video input format: {input_extension}"));
    }
    Ok(())
}

pub fn validate_conversion(
    input_path: &Path,
    output_path: &Path,
    format: VideoFormat,
    max_height: Option<u32>,
) -> Result<(), String> {
    validate_input(input_path)?;
    if !output_path.is_absolute() {
        return Err("Input and output paths must be absolute".into());
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
    if let Some(height) = max_height {
        if !(MIN_HEIGHT..=MAX_HEIGHT).contains(&height) {
            return Err(format!(
                "The maximum height must be between {MIN_HEIGHT} and {MAX_HEIGHT} pixels"
            ));
        }
    }
    Ok(())
}

pub fn validate_frames(
    input_path: &Path,
    output_directory: &Path,
    interval_seconds: u32,
) -> Result<(), String> {
    validate_input(input_path)?;
    if !output_directory.is_absolute() || !output_directory.is_dir() {
        return Err("The output directory does not exist".into());
    }
    if !(1..=MAX_FRAME_INTERVAL_SECONDS).contains(&interval_seconds) {
        return Err(format!(
            "The frame interval must be between 1 and {MAX_FRAME_INTERVAL_SECONDS} seconds"
        ));
    }
    Ok(())
}

pub fn convert<F>(
    request: VideoConversion<'_>,
    cancelled: Arc<AtomicBool>,
    mut report_progress: F,
) -> Result<ExecutionOutcome, String>
where
    F: FnMut(u8),
{
    validate_conversion(
        request.input_path,
        request.output_path,
        request.format,
        request.max_height,
    )?;
    if !request.ffmpeg_path.is_file() {
        return Err("The bundled FFmpeg executable could not be found".into());
    }
    for encoder in required_encoders(request.format) {
        if !ffmpeg::has_encoder(request.ffmpeg_path, encoder) {
            return Err(format!("This FFmpeg build has no {encoder} encoder"));
        }
    }

    let staged_path = ffmpeg::staged_output_path(request.output_path, request.job_id)?;
    let _ = fs::remove_file(&staged_path);
    let duration = request
        .ffprobe_path
        .and_then(|path| ffmpeg::probe_duration(path, request.input_path));

    let outcome = ffmpeg::run(
        request.ffmpeg_path,
        conversion_arguments(
            request.input_path,
            &staged_path,
            request.format,
            request.quality,
            request.max_height,
        ),
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

pub fn extract_frames<F>(
    request: FrameExtraction<'_>,
    cancelled: Arc<AtomicBool>,
    mut report_progress: F,
) -> Result<ExecutionOutcome, String>
where
    F: FnMut(u8),
{
    validate_frames(
        request.input_path,
        request.output_directory,
        request.interval_seconds,
    )?;
    if !request.ffmpeg_path.is_file() {
        return Err("The bundled FFmpeg executable could not be found".into());
    }

    let stem = request
        .input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or("The input filename is invalid")?;
    let staging = StagingDirectory(
        request
            .output_directory
            .join(format!(".{stem}.{}.frames", request.job_id)),
    );
    let _ = fs::remove_dir_all(&staging.0);
    fs::create_dir(&staging.0)
        .map_err(|error| format!("The staging directory could not be created: {error}"))?;

    let duration = request
        .ffprobe_path
        .and_then(|path| ffmpeg::probe_duration(path, request.input_path));
    let extension = request.format.extension();
    let pattern = staging.0.join(format!("frame-%06d.{extension}"));
    let outcome = ffmpeg::run(
        request.ffmpeg_path,
        frame_arguments(
            request.input_path,
            &pattern,
            request.format,
            request.interval_seconds,
        ),
        duration,
        &cancelled,
        |progress| report_progress(progress.min(95)),
    )?;
    if !matches!(outcome, ExecutionOutcome::Completed) {
        return Ok(outcome);
    }

    let mut frames: Vec<PathBuf> = fs::read_dir(&staging.0)
        .map_err(|error| format!("Extracted frames could not be listed: {error}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| fs::metadata(path).is_ok_and(|metadata| metadata.len() > 0))
        .collect();
    frames.sort();
    if frames.is_empty() {
        return Err("FFmpeg did not extract any frames".into());
    }

    let digits = frames.len().to_string().len().max(3);
    let mut finalized: Vec<PathBuf> = Vec::new();
    for (index, staged) in frames.iter().enumerate() {
        let final_path = next_available_path(
            request.output_directory,
            &format!("{stem}-frame-{:0digits$}", index + 1),
            extension,
        );
        if let Err(error) = fs::rename(staged, &final_path) {
            for path in finalized {
                let _ = fs::remove_file(path);
            }
            return Err(format!("Extracted frames could not be finalized: {error}"));
        }
        finalized.push(final_path);
    }
    report_progress(100);
    Ok(ExecutionOutcome::Completed)
}

struct StagingDirectory(PathBuf);

impl Drop for StagingDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
}

fn next_available_path(directory: &Path, stem: &str, extension: &str) -> PathBuf {
    let first = directory.join(format!("{stem}.{extension}"));
    if !first.exists() {
        return first;
    }
    let mut suffix = 2u32;
    loop {
        let candidate = directory.join(format!("{stem}-{suffix}.{extension}"));
        if !candidate.exists() {
            return candidate;
        }
        suffix = suffix.saturating_add(1);
    }
}

fn required_encoders(format: VideoFormat) -> &'static [&'static str] {
    match format {
        VideoFormat::Mp4 | VideoFormat::Mov | VideoFormat::Mkv => &["libx264", "aac"],
        VideoFormat::Webm => &["libvpx-vp9", "libopus"],
        VideoFormat::Gif => &["gif"],
    }
}

fn crf(format: VideoFormat, quality: VideoQuality) -> &'static str {
    match (format, quality) {
        (VideoFormat::Webm, VideoQuality::High) => "24",
        (VideoFormat::Webm, VideoQuality::Balanced) => "32",
        (VideoFormat::Webm, VideoQuality::Small) => "40",
        (_, VideoQuality::High) => "18",
        (_, VideoQuality::Balanced) => "23",
        (_, VideoQuality::Small) => "28",
    }
}

fn audio_bitrate(format: VideoFormat, quality: VideoQuality) -> &'static str {
    match (format, quality) {
        (VideoFormat::Webm, VideoQuality::High) => "128k",
        (VideoFormat::Webm, VideoQuality::Balanced) => "96k",
        (VideoFormat::Webm, VideoQuality::Small) => "64k",
        (_, VideoQuality::High) => "192k",
        (_, VideoQuality::Balanced) => "128k",
        (_, VideoQuality::Small) => "96k",
    }
}

fn gif_settings(quality: VideoQuality) -> (u32, u32) {
    match quality {
        VideoQuality::High => (15, 720),
        VideoQuality::Balanced => (12, 480),
        VideoQuality::Small => (10, 360),
    }
}

// Even dimensions are required by yuv420p; `-2` keeps the aspect ratio and rounds to even.
fn scale_filter(max_height: Option<u32>) -> String {
    match max_height {
        Some(height) => format!("scale=-2:'min({height},ih)'"),
        None => "scale=trunc(iw/2)*2:trunc(ih/2)*2".into(),
    }
}

fn conversion_arguments(
    input: &Path,
    output: &Path,
    format: VideoFormat,
    quality: VideoQuality,
    max_height: Option<u32>,
) -> Vec<OsString> {
    let mut arguments = ffmpeg::base_arguments(input);
    let mut push = |values: &[&str]| arguments.extend(values.iter().map(OsString::from));

    if format == VideoFormat::Gif {
        let (fps, default_height) = gif_settings(quality);
        let height = max_height.unwrap_or(default_height);
        push(&[
            "-filter_complex",
            &format!(
                "[0:v:0]fps={fps},scale=-1:'min({height},ih)':flags=lanczos,split[a][b];[a]palettegen[p];[b][p]paletteuse[out]"
            ),
            "-map",
            "[out]",
            "-loop",
            "0",
        ]);
    } else {
        push(&[
            "-map",
            "0:v:0",
            "-map",
            "0:a:0?",
            "-sn",
            "-dn",
            "-map_metadata",
            "0",
            "-vf",
            &scale_filter(max_height),
            "-pix_fmt",
            "yuv420p",
        ]);
        let crf = crf(format, quality);
        let audio = audio_bitrate(format, quality);
        match format {
            VideoFormat::Webm => push(&[
                "-c:v",
                "libvpx-vp9",
                "-crf",
                crf,
                "-b:v",
                "0",
                "-deadline",
                "good",
                "-cpu-used",
                "4",
                "-row-mt",
                "1",
                "-c:a",
                "libopus",
                "-b:a",
                audio,
            ]),
            _ => push(&[
                "-c:v", "libx264", "-preset", "medium", "-crf", crf, "-c:a", "aac", "-b:a", audio,
            ]),
        }
        if matches!(format, VideoFormat::Mp4 | VideoFormat::Mov) {
            push(&["-movflags", "+faststart"]);
        }
    }

    arguments.extend(ffmpeg::progress_arguments());
    arguments.push(output.as_os_str().to_owned());
    arguments
}

fn frame_arguments(
    input: &Path,
    pattern: &Path,
    format: PdfImageFormat,
    interval_seconds: u32,
) -> Vec<OsString> {
    let mut arguments = ffmpeg::base_arguments(input);
    arguments.extend(
        [
            "-map",
            "0:v:0",
            "-an",
            "-sn",
            "-vf",
            &format!("fps=1/{interval_seconds}"),
        ]
        .map(OsString::from),
    );
    if format == PdfImageFormat::Jpeg {
        arguments.extend(["-q:v", "2"].map(OsString::from));
    }
    arguments.extend(ffmpeg::progress_arguments());
    arguments.push(pattern.as_os_str().to_owned());
    arguments
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform;
    use tempfile::tempdir;

    fn rendered(arguments: &[OsString]) -> Vec<String> {
        arguments
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect()
    }

    fn has_pair(arguments: &[String], key: &str, value: &str) -> bool {
        arguments.windows(2).any(|pair| pair == [key, value])
    }

    #[test]
    fn mp4_uses_h264_aac_with_the_preset_crf_and_faststart() {
        let args = rendered(&conversion_arguments(
            Path::new("/v/in.mov"),
            Path::new("/v/out.mp4"),
            VideoFormat::Mp4,
            VideoQuality::Small,
            None,
        ));

        assert!(has_pair(&args, "-c:v", "libx264"));
        assert!(has_pair(&args, "-crf", "28"));
        assert!(has_pair(&args, "-c:a", "aac"));
        assert!(has_pair(&args, "-movflags", "+faststart"));
        assert!(has_pair(&args, "-vf", "scale=trunc(iw/2)*2:trunc(ih/2)*2"));
        assert_eq!(args.last().map(String::as_str), Some("/v/out.mp4"));
    }

    #[test]
    fn webm_uses_vp9_opus_with_constant_quality() {
        let args = rendered(&conversion_arguments(
            Path::new("/v/in.mp4"),
            Path::new("/v/out.webm"),
            VideoFormat::Webm,
            VideoQuality::High,
            Some(720),
        ));

        assert!(has_pair(&args, "-c:v", "libvpx-vp9"));
        assert!(has_pair(&args, "-crf", "24"));
        assert!(has_pair(&args, "-b:v", "0"));
        assert!(has_pair(&args, "-c:a", "libopus"));
        assert!(has_pair(&args, "-vf", "scale=-2:'min(720,ih)'"));
        assert!(!args.contains(&"-movflags".to_string()));
    }

    #[test]
    fn mkv_skips_faststart_and_drops_subtitles() {
        let args = rendered(&conversion_arguments(
            Path::new("/v/in.mp4"),
            Path::new("/v/out.mkv"),
            VideoFormat::Mkv,
            VideoQuality::Balanced,
            None,
        ));

        assert!(has_pair(&args, "-crf", "23"));
        assert!(args.contains(&"-sn".to_string()));
        assert!(!args.contains(&"-movflags".to_string()));
    }

    #[test]
    fn gif_builds_a_palette_graph_without_audio() {
        let args = rendered(&conversion_arguments(
            Path::new("/v/in.mp4"),
            Path::new("/v/out.gif"),
            VideoFormat::Gif,
            VideoQuality::Balanced,
            None,
        ));

        let graph = args
            .iter()
            .find(|value| value.contains("palettegen"))
            .expect("palette graph");
        assert!(graph.contains("fps=12"));
        assert!(graph.contains("min(480,ih)"));
        assert!(graph.contains("paletteuse"));
        assert!(has_pair(&args, "-map", "[out]"));
        assert!(!args.contains(&"-c:a".to_string()));
    }

    #[test]
    fn frame_extraction_samples_one_frame_per_interval() {
        let args = rendered(&frame_arguments(
            Path::new("/v/in.mp4"),
            Path::new("/v/.in.1.frames/frame-%06d.jpg"),
            PdfImageFormat::Jpeg,
            5,
        ));

        assert!(has_pair(&args, "-vf", "fps=1/5"));
        assert!(has_pair(&args, "-q:v", "2"));
        assert!(args.contains(&"-an".to_string()));
    }

    #[test]
    fn validation_rejects_bad_requests() {
        let directory = tempdir().unwrap();
        let input = directory.path().join("clip.mp4");
        let existing = directory.path().join("existing.mp4");
        fs::write(&input, b"input").unwrap();
        fs::write(&existing, b"existing").unwrap();
        let output = directory.path().join("clip.webm");

        assert!(
            validate_conversion(&input, &existing, VideoFormat::Mp4, None)
                .unwrap_err()
                .contains("already exists")
        );
        assert!(validate_conversion(&input, &output, VideoFormat::Mp4, None)
            .unwrap_err()
            .contains("must be .mp4"));
        assert!(
            validate_conversion(&input, &output, VideoFormat::Webm, Some(10))
                .unwrap_err()
                .contains("maximum height")
        );
        assert!(validate_conversion(&input, &output, VideoFormat::Webm, Some(720)).is_ok());

        let document = directory.path().join("notes.txt");
        fs::write(&document, b"x").unwrap();
        assert!(validate_frames(&document, directory.path(), 1)
            .unwrap_err()
            .contains("Unsupported"));
        assert!(validate_frames(&input, directory.path(), 0)
            .unwrap_err()
            .contains("interval"));
        assert!(validate_frames(&input, directory.path(), 2).is_ok());
    }

    fn generate_clip(ffmpeg: &Path, path: &Path, seconds: u32) {
        let status = platform::background_command(ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i"])
            .arg(format!("testsrc=size=320x240:rate=15:duration={seconds}"))
            .args(["-f", "lavfi", "-i"])
            .arg(format!("sine=frequency=440:duration={seconds}"))
            .args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac"])
            .arg(path)
            .status()
            .unwrap();
        assert!(status.success());
    }

    fn probe_codec(ffprobe: &Path, path: &Path, stream: &str) -> String {
        let output = platform::background_command(ffprobe)
            .args(["-v", "error", "-select_streams", stream])
            .args(["-show_entries", "stream=codec_name"])
            .args(["-of", "default=noprint_wrappers=1:nokey=1"])
            .arg(path)
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    #[test]
    #[ignore = "runs the bundled FFmpeg binary"]
    fn bundled_engine_converts_every_video_format() {
        let directory = tempdir().unwrap();
        let ffmpeg = platform::find_ffmpeg(None).expect("FFmpeg is required for this smoke test");
        let ffprobe = platform::find_ffprobe(None).expect("ffprobe is required");
        let input = directory.path().join("source.mp4");
        generate_clip(&ffmpeg, &input, 2);

        for (index, (format, video, audio)) in [
            (VideoFormat::Mp4, "h264", "aac"),
            (VideoFormat::Webm, "vp9", "opus"),
            (VideoFormat::Mov, "h264", "aac"),
            (VideoFormat::Mkv, "h264", "aac"),
            (VideoFormat::Gif, "gif", ""),
        ]
        .into_iter()
        .enumerate()
        {
            let output = directory
                .path()
                .join(format!("output-{index}.{}", format.extension()));
            let mut last_progress = 0;
            let outcome = convert(
                VideoConversion {
                    ffmpeg_path: &ffmpeg,
                    ffprobe_path: Some(&ffprobe),
                    input_path: &input,
                    output_path: &output,
                    format,
                    quality: VideoQuality::Small,
                    max_height: Some(144),
                    job_id: &index.to_string(),
                },
                Arc::new(AtomicBool::new(false)),
                |progress| last_progress = progress,
            )
            .unwrap();

            assert!(matches!(outcome, ExecutionOutcome::Completed));
            assert_eq!(last_progress, 100);
            assert_eq!(probe_codec(&ffprobe, &output, "v:0"), video);
            assert_eq!(probe_codec(&ffprobe, &output, "a:0"), audio);
            let height = platform::background_command(&ffprobe)
                .args(["-v", "error", "-select_streams", "v:0"])
                .args(["-show_entries", "stream=height"])
                .args(["-of", "default=noprint_wrappers=1:nokey=1"])
                .arg(&output)
                .output()
                .unwrap();
            assert_eq!(String::from_utf8_lossy(&height.stdout).trim(), "144");
        }
        let leftovers: Vec<_> = fs::read_dir(directory.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().starts_with('.'))
            .collect();
        assert!(leftovers.is_empty());
    }

    #[test]
    #[ignore = "runs the bundled FFmpeg binary"]
    fn bundled_engine_extracts_frames_and_cleans_up_staging() {
        let directory = tempdir().unwrap();
        let ffmpeg = platform::find_ffmpeg(None).expect("FFmpeg is required for this smoke test");
        let input = directory.path().join("clip.mp4");
        generate_clip(&ffmpeg, &input, 5);
        let output = directory.path().join("frames");
        fs::create_dir(&output).unwrap();

        let outcome = extract_frames(
            FrameExtraction {
                ffmpeg_path: &ffmpeg,
                ffprobe_path: platform::find_ffprobe(None).as_deref(),
                input_path: &input,
                output_directory: &output,
                format: PdfImageFormat::Png,
                interval_seconds: 2,
                job_id: "7",
            },
            Arc::new(AtomicBool::new(false)),
            |_| {},
        )
        .unwrap();

        assert!(matches!(outcome, ExecutionOutcome::Completed));
        let mut names: Vec<_> = fs::read_dir(&output)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(
            names,
            [
                "clip-frame-001.png",
                "clip-frame-002.png",
                "clip-frame-003.png"
            ]
        );
    }

    #[test]
    #[ignore = "runs the bundled FFmpeg binary"]
    fn cancelled_conversion_leaves_no_files_behind() {
        let directory = tempdir().unwrap();
        let ffmpeg = platform::find_ffmpeg(None).expect("FFmpeg is required for this smoke test");
        let input = directory.path().join("clip.mp4");
        generate_clip(&ffmpeg, &input, 2);
        let output = directory.path().join("clip.webm");

        let outcome = convert(
            VideoConversion {
                ffmpeg_path: &ffmpeg,
                ffprobe_path: None,
                input_path: &input,
                output_path: &output,
                format: VideoFormat::Webm,
                quality: VideoQuality::High,
                max_height: None,
                job_id: "9",
            },
            Arc::new(AtomicBool::new(true)),
            |_| {},
        )
        .unwrap();

        assert!(matches!(outcome, ExecutionOutcome::Cancelled));
        assert!(!output.exists());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}

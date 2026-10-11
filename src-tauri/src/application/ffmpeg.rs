use std::{
    ffi::OsString,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use crate::{domain::job::ExecutionOutcome, platform};

pub fn staged_output_path(output_path: &Path, job_id: &str) -> Result<PathBuf, String> {
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

pub fn has_encoder(ffmpeg: &Path, name: &str) -> bool {
    platform::background_command(ffmpeg)
        .args(["-hide_banner", "-encoders"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|output| output.status.success())
        .is_some_and(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .any(|line| line.split_whitespace().nth(1) == Some(name))
        })
}

pub fn probe_duration(ffprobe: &Path, input: &Path) -> Option<f64> {
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
    String::from_utf8(output.stdout)
        .ok()?
        .trim()
        .parse()
        .ok()
        .filter(|duration: &f64| duration.is_finite() && *duration > 0.0)
}

/// Runs FFmpeg to completion, reporting progress from `-progress pipe:1` against `duration`.
/// The caller removes any staged output when this returns an error or `Cancelled`.
pub fn run<F>(
    ffmpeg: &Path,
    arguments: Vec<OsString>,
    duration: Option<f64>,
    cancelled: &AtomicBool,
    mut report_progress: F,
) -> Result<ExecutionOutcome, String>
where
    F: FnMut(u8),
{
    let mut command = platform::background_command(ffmpeg);
    command
        .args(arguments)
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
                return Err(format!("FFmpeg process monitoring failed: {error}"));
            }
        }
    };

    let _ = progress_thread.join();
    let error_message = error_thread.join().unwrap_or_default();

    let Some(status) = status else {
        return Ok(ExecutionOutcome::Cancelled);
    };
    if !status.success() {
        let detail = error_message.trim();
        return Err(if detail.is_empty() {
            format!("FFmpeg exited with status {status}")
        } else {
            format!("FFmpeg conversion failed: {detail}")
        });
    }
    Ok(ExecutionOutcome::Completed)
}

pub fn base_arguments(input: &Path) -> Vec<OsString> {
    vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-nostdin".into(),
        "-n".into(),
        "-i".into(),
        input.as_os_str().to_owned(),
    ]
}

pub fn progress_arguments() -> [OsString; 3] {
    ["-progress".into(), "pipe:1".into(), "-nostats".into()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staging_keeps_the_real_output_extension() {
        let output = Path::new("C:/audio/output.ogg");
        assert_eq!(
            staged_output_path(output, "42").unwrap(),
            PathBuf::from("C:/audio/.output.42.part.ogg")
        );
    }
}

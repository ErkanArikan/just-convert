use std::{
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

use crate::{domain::job::ExecutionOutcome, platform};

const PROGRESS_PREFIX: &str = "FILE_CONVERTER_PROGRESS:";

pub fn validate_request(input_path: &Path, output_path: &Path) -> Result<(), String> {
    if !input_path.is_absolute() || !input_path.is_file() {
        return Err(format!(
            "PDF input does not exist: {}",
            input_path.display()
        ));
    }
    if extension(input_path).as_deref() != Some("pdf") {
        return Err("PDF-to-Word input must use the .pdf extension".into());
    }
    if !output_path.is_absolute() {
        return Err("DOCX output path must be absolute".into());
    }
    if output_path.exists() {
        return Err("The DOCX output file already exists".into());
    }
    if extension(output_path).as_deref() != Some("docx") {
        return Err("The output file must use the .docx extension".into());
    }
    if output_path.parent().is_none_or(|parent| !parent.is_dir()) {
        return Err("The DOCX output directory does not exist".into());
    }
    if input_path == output_path {
        return Err("Input and output paths must be different".into());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn convert<F>(
    python_path: &Path,
    worker_path: &Path,
    input_path: &Path,
    output_path: &Path,
    job_id: &str,
    cancelled: Arc<AtomicBool>,
    mut report_progress: F,
) -> Result<ExecutionOutcome, String>
where
    F: FnMut(u8),
{
    validate_request(input_path, output_path)?;
    if !python_path.is_file() {
        return Err("Python 3.10 or newer could not be found".into());
    }
    if !worker_path.is_file() {
        return Err("The PDF-to-Word worker script could not be found".into());
    }

    let staged_path = staged_output_path(output_path, job_id)?;
    let _ = fs::remove_file(&staged_path);
    let mut child = platform::background_command(python_path)
        .arg("-B")
        .arg(worker_path)
        .arg(input_path)
        .arg(&staged_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Python PDF-to-Word worker could not be started: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or("Python worker output pipe unavailable")?;
    let stderr = child
        .stderr
        .take()
        .ok_or("Python worker error pipe unavailable")?;
    let (progress_tx, progress_rx) = mpsc::channel();
    let progress_thread = thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Some(value) = line.strip_prefix(PROGRESS_PREFIX) {
                if let Ok(value) = value.parse::<u8>() {
                    let _ = progress_tx.send(value.min(100));
                }
            }
        }
    });
    let error_thread = thread::spawn(move || {
        let mut message = String::new();
        let _ = BufReader::new(stderr).read_to_string(&mut message);
        message
    });

    let status = loop {
        for progress in progress_rx.try_iter() {
            report_progress(progress);
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
                return Err(format!("Python worker monitoring failed: {error}"));
            }
        }
    };
    let _ = progress_thread.join();
    for progress in progress_rx.try_iter() {
        report_progress(progress);
    }
    let error_message = error_thread.join().unwrap_or_default();

    let Some(status) = status else {
        let _ = fs::remove_file(staged_path);
        return Ok(ExecutionOutcome::Cancelled);
    };
    if !status.success() {
        let _ = fs::remove_file(staged_path);
        let detail = error_message.trim();
        return Err(if detail.is_empty() {
            format!("Python PDF-to-Word worker exited with status {status}")
        } else {
            detail.to_string()
        });
    }
    validate_generated_docx(&staged_path)?;
    if cancelled.load(Ordering::Relaxed) {
        let _ = fs::remove_file(staged_path);
        return Ok(ExecutionOutcome::Cancelled);
    }
    if let Err(error) = fs::rename(&staged_path, output_path) {
        let _ = fs::remove_file(staged_path);
        return Err(format!("Converted DOCX could not be finalized: {error}"));
    }
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
        .ok_or("DOCX output directory is unavailable")?;
    let stem = output_path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or("DOCX output filename is invalid")?;
    Ok(parent.join(format!(".{stem}.{job_id}.part.docx")))
}

fn validate_generated_docx(path: &Path) -> Result<(), String> {
    if fs::metadata(path).map_or(true, |metadata| metadata.len() < 4) {
        let _ = fs::remove_file(path);
        return Err("The Python worker did not produce a valid DOCX file".into());
    }
    let mut signature = [0_u8; 4];
    fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut signature))
        .map_err(|error| format!("Generated DOCX could not be validated: {error}"))?;
    if signature != [0x50, 0x4b, 0x03, 0x04] {
        let _ = fs::remove_file(path);
        return Err("The Python worker produced an invalid DOCX archive".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform;
    use tempfile::tempdir;

    #[test]
    fn validation_rejects_non_pdf_inputs_and_existing_outputs() {
        let directory = tempdir().unwrap();
        let input = directory.path().join("notes.txt");
        let output = directory.path().join("notes.docx");
        fs::write(&input, b"text").unwrap();
        assert!(validate_request(&input, &output)
            .unwrap_err()
            .contains(".pdf"));

        let input = directory.path().join("notes.pdf");
        fs::write(&input, b"%PDF-1.7").unwrap();
        fs::write(&output, b"existing").unwrap();
        assert!(validate_request(&input, &output)
            .unwrap_err()
            .contains("already exists"));
    }

    #[test]
    fn staging_path_keeps_a_docx_extension() {
        assert_eq!(
            staged_output_path(Path::new("C:/files/report.docx"), "42").unwrap(),
            PathBuf::from("C:/files/.report.42.part.docx")
        );
    }

    #[test]
    fn external_python_process_boundary_can_publish_a_staged_docx() {
        let Some(python) = platform::find_python() else {
            return;
        };
        let directory = tempdir().unwrap();
        let input = directory.path().join("source.pdf");
        let output = directory.path().join("result.docx");
        let worker = directory.path().join("fake-worker.py");
        fs::write(&input, b"%PDF-1.7\n").unwrap();
        fs::write(
            &worker,
            "import sys, zipfile\nprint('FILE_CONVERTER_PROGRESS:50', flush=True)\nwith zipfile.ZipFile(sys.argv[2], 'w') as z: z.writestr('[Content_Types].xml', '<Types/>')\n",
        )
        .unwrap();

        let mut progress = Vec::new();
        let outcome = convert(
            &python,
            &worker,
            &input,
            &output,
            "test",
            Arc::new(AtomicBool::new(false)),
            |value| progress.push(value),
        )
        .unwrap();

        assert!(matches!(outcome, ExecutionOutcome::Completed));
        assert!(output.is_file());
        assert!(progress.contains(&50));
        assert_eq!(progress.last(), Some(&100));
    }

    #[test]
    #[ignore = "runs installed Python, PyMuPDF, and pdf2docx"]
    fn installed_python_worker_converts_selectable_text_pdf_to_docx() {
        let runtime = platform::find_pdf_to_docx_runtime(None)
            .expect("Python, PyMuPDF, and pdf2docx are required for this smoke test");
        let directory = tempdir().unwrap();
        let input = directory.path().join("selectable-text.pdf");
        let output = directory.path().join("converted.docx");
        let generated = platform::background_command(&runtime.python_path)
            .args([
                "-c",
                "import pymupdf, sys; doc=pymupdf.open(); page=doc.new_page(); page.insert_text((72, 72), 'Local PDF to Word smoke test'); doc.save(sys.argv[1]); doc.close()",
            ])
            .arg(&input)
            .status()
            .unwrap();
        assert!(generated.success());

        let outcome = convert(
            &runtime.python_path,
            &runtime.worker_path,
            &input,
            &output,
            "real-worker",
            Arc::new(AtomicBool::new(false)),
            |_| {},
        )
        .unwrap();

        assert!(matches!(outcome, ExecutionOutcome::Completed));
        assert!(fs::metadata(output).unwrap().len() > 0);
    }

    #[test]
    #[ignore = "runs installed Python, PyMuPDF, and pdf2docx"]
    fn installed_python_worker_rejects_pdf_without_selectable_text() {
        let runtime = platform::find_pdf_to_docx_runtime(None)
            .expect("Python, PyMuPDF, and pdf2docx are required for this smoke test");
        let directory = tempdir().unwrap();
        let input = directory.path().join("image-only.pdf");
        let output = directory.path().join("converted.docx");
        let generated = platform::background_command(&runtime.python_path)
            .args([
                "-c",
                "import pymupdf, sys; doc=pymupdf.open(); doc.new_page(); doc.save(sys.argv[1]); doc.close()",
            ])
            .arg(&input)
            .status()
            .unwrap();
        assert!(generated.success());

        let error = convert(
            &runtime.python_path,
            &runtime.worker_path,
            &input,
            &output,
            "image-only",
            Arc::new(AtomicBool::new(false)),
            |_| {},
        )
        .unwrap_err();

        assert!(error.contains("no selectable text"));
        assert!(!output.exists());
    }
}

use std::{
    fs,
    io::{BufReader, Read},
    path::{Path, PathBuf},
    process::{ExitStatus, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use url::Url;

use crate::{application::pdf_converter, domain::job::ExecutionOutcome, platform};

const SUPPORTED_EXTENSIONS: &[&str] = &["docx", "xlsx", "pptx"];

pub fn is_supported_input(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| {
            SUPPORTED_EXTENSIONS
                .iter()
                .any(|extension| value.eq_ignore_ascii_case(extension))
        })
}

struct ProcessOutput {
    status: ExitStatus,
    stdout: String,
    stderr: String,
}

pub fn validate_request(input_path: &Path, output_path: &Path) -> Result<(), String> {
    if !input_path.is_absolute() || !input_path.is_file() {
        return Err(format!(
            "Office input does not exist: {}",
            input_path.display()
        ));
    }
    if !is_supported_input(input_path) {
        return Err("Office input must be a DOCX, XLSX, or PPTX file".into());
    }
    if !output_path.is_absolute() {
        return Err("PDF output path must be absolute".into());
    }
    if output_path.exists() {
        return Err("The PDF output file already exists".into());
    }
    if output_path
        .extension()
        .and_then(|value| value.to_str())
        .is_none_or(|value| !value.eq_ignore_ascii_case("pdf"))
    {
        return Err("The output file must use the .pdf extension".into());
    }
    if output_path.parent().is_none_or(|parent| !parent.is_dir()) {
        return Err("The PDF output directory does not exist".into());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn convert<F>(
    libreoffice_path: &Path,
    qpdf_path: &Path,
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
    if !libreoffice_path.is_file() {
        return Err("LibreOffice could not be found".into());
    }
    let staging = staging_directory(output_path, job_id)?;
    if staging.exists() {
        return Err("Office conversion staging directory already exists".into());
    }
    let output_directory = staging.join("output");
    let profile_directory = staging.join("profile");
    fs::create_dir_all(&output_directory)
        .and_then(|_| fs::create_dir_all(&profile_directory))
        .map_err(|error| format!("Office conversion staging could not be created: {error}"))?;
    let profile_url = Url::from_directory_path(&profile_directory)
        .map_err(|_| "LibreOffice profile path could not be encoded")?;
    report_progress(5);

    let mut child = platform::background_command(libreoffice_path)
        .args([
            "--headless",
            "--nologo",
            "--nodefault",
            "--nolockcheck",
            "--nofirststartwizard",
        ])
        .arg(format!("-env:UserInstallation={profile_url}"))
        .args(["--convert-to", "pdf", "--outdir"])
        .arg(&output_directory)
        .arg(input_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            let _ = fs::remove_dir_all(&staging);
            format!("LibreOffice could not be started: {error}")
        })?;
    let stdout = child
        .stdout
        .take()
        .ok_or("LibreOffice output pipe unavailable")?;
    let stderr = child
        .stderr
        .take()
        .ok_or("LibreOffice error pipe unavailable")?;
    let stdout_thread = thread::spawn(move || read_pipe(stdout));
    let stderr_thread = thread::spawn(move || read_pipe(stderr));

    let status = loop {
        if cancelled.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = fs::remove_dir_all(&staging);
                return Err(format!("LibreOffice process monitoring failed: {error}"));
            }
        }
    };
    let stdout = stdout_thread.join().unwrap_or_default();
    let stderr = stderr_thread.join().unwrap_or_default();
    let Some(status) = status else {
        let _ = fs::remove_dir_all(&staging);
        return Ok(ExecutionOutcome::Cancelled);
    };
    let process_output = ProcessOutput {
        status,
        stdout,
        stderr,
    };
    if !process_output.status.success() {
        let error = process_error(&process_output);
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    report_progress(75);

    let generated = generated_output_path(input_path, &output_directory)?;
    if !generated.is_file() {
        let error = process_error(&process_output);
        let _ = fs::remove_dir_all(&staging);
        return Err(format!("LibreOffice did not produce a PDF. {error}"));
    }
    match pdf_converter::inspect_page_count(qpdf_path, &generated) {
        Ok(count) if count > 0 => {}
        Ok(_) => {
            let _ = fs::remove_dir_all(&staging);
            return Err("LibreOffice produced a PDF with no pages".into());
        }
        Err(error) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(format!("LibreOffice PDF validation failed: {error}"));
        }
    };
    report_progress(90);
    if cancelled.load(Ordering::Relaxed) {
        let _ = fs::remove_dir_all(&staging);
        return Ok(ExecutionOutcome::Cancelled);
    }
    fs::rename(&generated, output_path)
        .map_err(|error| format!("Office PDF could not be finalized: {error}"))?;
    let _ = fs::remove_dir_all(&staging);
    report_progress(100);
    Ok(ExecutionOutcome::Completed)
}

fn staging_directory(output_path: &Path, job_id: &str) -> Result<PathBuf, String> {
    let parent = output_path
        .parent()
        .ok_or("PDF output directory is unavailable")?;
    Ok(parent.join(format!(".just-convert-office-{job_id}")))
}

fn generated_output_path(input_path: &Path, output_directory: &Path) -> Result<PathBuf, String> {
    let stem = input_path
        .file_stem()
        .ok_or("Office input filename is invalid")?;
    Ok(output_directory.join(stem).with_extension("pdf"))
}

fn read_pipe(pipe: impl Read) -> String {
    let mut value = String::new();
    let _ = BufReader::new(pipe).read_to_string(&mut value);
    value
}

fn process_error(output: &ProcessOutput) -> String {
    let detail = [output.stderr.trim(), output.stdout.trim()]
        .into_iter()
        .find(|value| !value.is_empty())
        .unwrap_or("No diagnostic output was provided");
    format!("LibreOffice conversion failed: {detail}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform;
    use tempfile::tempdir;

    #[test]
    fn unsupported_office_formats_are_rejected() {
        let directory = tempdir().unwrap();
        let input = directory.path().join("notes.txt");
        fs::write(&input, b"text").unwrap();
        assert!(
            validate_request(&input, &directory.path().join("notes.pdf"))
                .unwrap_err()
                .contains("DOCX, XLSX, or PPTX")
        );
    }

    #[test]
    fn generated_name_uses_the_source_stem() {
        assert_eq!(
            generated_output_path(Path::new("C:/files/report.docx"), Path::new("C:/out")).unwrap(),
            PathBuf::from("C:/out/report.pdf")
        );
    }

    #[test]
    #[ignore = "runs installed LibreOffice and bundled qpdf"]
    fn installed_libreoffice_converts_docx_to_valid_pdf() {
        let directory = tempdir().unwrap();
        let libreoffice =
            platform::find_libreoffice().expect("LibreOffice is required for this smoke test");
        let qpdf = platform::find_qpdf(None).expect("qpdf is required for this smoke test");
        let html = directory.path().join("office-smoke.html");
        fs::write(
            &html,
            b"<html><body><h1>Local conversion</h1><p>Smoke test</p></body></html>",
        )
        .unwrap();
        let fixture_profile = directory.path().join("fixture-profile");
        fs::create_dir(&fixture_profile).unwrap();
        let fixture_profile_url = Url::from_directory_path(&fixture_profile).unwrap();
        let fixture_status = platform::background_command(&libreoffice)
            .args(["--headless", "--nologo", "--nofirststartwizard"])
            .arg(format!("-env:UserInstallation={fixture_profile_url}"))
            .args(["--convert-to", "docx:Office Open XML Text", "--outdir"])
            .arg(directory.path())
            .arg(&html)
            .status()
            .unwrap();
        assert!(fixture_status.success());

        let docx = directory.path().join("office-smoke.docx");
        let output = directory.path().join("converted.pdf");
        assert!(docx.is_file());
        assert!(matches!(
            convert(
                &libreoffice,
                &qpdf,
                &docx,
                &output,
                "test",
                Arc::new(AtomicBool::new(false)),
                |_| {}
            )
            .unwrap(),
            ExecutionOutcome::Completed
        ));
        assert!(pdf_converter::inspect_page_count(&qpdf, &output).unwrap() > 0);
    }
}

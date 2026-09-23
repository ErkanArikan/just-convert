use std::{
    collections::HashSet,
    ffi::OsString,
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

use crate::{
    domain::job::{ExecutionOutcome, PdfOperation},
    platform,
};

struct ProcessOutput {
    status: ExitStatus,
    stdout: String,
    stderr: String,
}

enum ProcessOutcome {
    Finished(ProcessOutput),
    Cancelled,
}

pub fn validate_request(operation: &PdfOperation) -> Result<(), String> {
    match operation {
        PdfOperation::Merge {
            input_paths,
            output_path,
        } => {
            if input_paths.len() < 2 {
                return Err("PDF merge requires at least two input files".into());
            }
            for input in input_paths {
                validate_pdf(Path::new(input))?;
            }
            validate_output(Path::new(output_path))
        }
        PdfOperation::Split {
            input_path,
            output_directory,
            pages_per_file,
        } => {
            validate_pdf(Path::new(input_path))?;
            let directory = Path::new(output_directory);
            if !directory.is_absolute() || !directory.is_dir() {
                return Err("The split output directory does not exist".into());
            }
            if !(1..=1000).contains(pages_per_file) {
                return Err("Pages per file must be between 1 and 1000".into());
            }
            Ok(())
        }
        PdfOperation::Reorder {
            input_path,
            output_path,
            page_order,
        } => {
            validate_pdf(Path::new(input_path))?;
            validate_output(Path::new(output_path))?;
            validate_page_list(page_order, false)
        }
        PdfOperation::Rotate {
            input_path,
            output_path,
            angle,
            pages,
        } => {
            validate_pdf(Path::new(input_path))?;
            validate_output(Path::new(output_path))?;
            if ![90, 180, 270].contains(angle) {
                return Err("Rotation angle must be 90, 180, or 270 degrees".into());
            }
            validate_page_list(pages, true)
        }
    }
}

pub fn execute<F>(
    qpdf_path: &Path,
    operation: &PdfOperation,
    job_id: &str,
    cancelled: Arc<AtomicBool>,
    mut report_progress: F,
) -> Result<ExecutionOutcome, String>
where
    F: FnMut(u8),
{
    validate_request(operation)?;
    if !qpdf_path.is_file() {
        return Err("The bundled qpdf executable could not be found".into());
    }

    match operation {
        PdfOperation::Merge {
            input_paths,
            output_path,
        } => execute_merge(
            qpdf_path,
            input_paths,
            Path::new(output_path),
            job_id,
            cancelled,
            &mut report_progress,
        ),
        PdfOperation::Split {
            input_path,
            output_directory,
            pages_per_file,
        } => execute_split(
            qpdf_path,
            Path::new(input_path),
            Path::new(output_directory),
            *pages_per_file,
            job_id,
            cancelled,
            &mut report_progress,
        ),
        PdfOperation::Reorder {
            input_path,
            output_path,
            page_order,
        } => execute_reorder(
            qpdf_path,
            Path::new(input_path),
            Path::new(output_path),
            page_order,
            job_id,
            cancelled,
            &mut report_progress,
        ),
        PdfOperation::Rotate {
            input_path,
            output_path,
            angle,
            pages,
        } => execute_rotate(
            qpdf_path,
            Path::new(input_path),
            Path::new(output_path),
            *angle,
            pages,
            job_id,
            cancelled,
            &mut report_progress,
        ),
    }
}

pub fn inspect_page_count(qpdf_path: &Path, input_path: &Path) -> Result<u32, String> {
    validate_pdf(input_path)?;
    if !qpdf_path.is_file() {
        return Err("The bundled qpdf executable could not be found".into());
    }
    page_count(qpdf_path, input_path, Arc::new(AtomicBool::new(false)))?
        .ok_or_else(|| "PDF inspection was cancelled".into())
}

fn execute_merge<F>(
    qpdf: &Path,
    inputs: &[String],
    output: &Path,
    job_id: &str,
    cancelled: Arc<AtomicBool>,
    progress: &mut F,
) -> Result<ExecutionOutcome, String>
where
    F: FnMut(u8),
{
    let staged = staged_output_path(output, job_id)?;
    let mut args = vec![OsString::from("--empty"), OsString::from("--pages")];
    for input in inputs {
        args.push(OsString::from(input));
        args.push(OsString::from("1-z"));
    }
    args.push(OsString::from("--"));
    args.push(staged.as_os_str().to_owned());
    run_single_output(qpdf, args, &staged, output, cancelled, progress)
}

fn execute_reorder<F>(
    qpdf: &Path,
    input: &Path,
    output: &Path,
    page_order: &[u32],
    job_id: &str,
    cancelled: Arc<AtomicBool>,
    progress: &mut F,
) -> Result<ExecutionOutcome, String>
where
    F: FnMut(u8),
{
    let Some(page_count) = page_count(qpdf, input, Arc::clone(&cancelled))? else {
        return Ok(ExecutionOutcome::Cancelled);
    };
    let mut sorted = page_order.to_vec();
    sorted.sort_unstable();
    if sorted != (1..=page_count).collect::<Vec<_>>() {
        return Err(format!(
            "Page order must contain every page from 1 through {page_count} exactly once"
        ));
    }
    let staged = staged_output_path(output, job_id)?;
    let page_spec = page_order
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let args = vec![
        input.as_os_str().to_owned(),
        OsString::from("--pages"),
        OsString::from("."),
        OsString::from(page_spec),
        OsString::from("--"),
        staged.as_os_str().to_owned(),
    ];
    run_single_output(qpdf, args, &staged, output, cancelled, progress)
}

#[allow(clippy::too_many_arguments)]
fn execute_rotate<F>(
    qpdf: &Path,
    input: &Path,
    output: &Path,
    angle: u16,
    pages: &[u32],
    job_id: &str,
    cancelled: Arc<AtomicBool>,
    progress: &mut F,
) -> Result<ExecutionOutcome, String>
where
    F: FnMut(u8),
{
    if !pages.is_empty() {
        let Some(count) = page_count(qpdf, input, Arc::clone(&cancelled))? else {
            return Ok(ExecutionOutcome::Cancelled);
        };
        if pages.iter().any(|page| *page > count) {
            return Err(format!("Rotation pages must be between 1 and {count}"));
        }
    }
    let staged = staged_output_path(output, job_id)?;
    let page_suffix = if pages.is_empty() {
        String::new()
    } else {
        format!(
            ":{}",
            pages
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    let args = vec![
        input.as_os_str().to_owned(),
        staged.as_os_str().to_owned(),
        OsString::from(format!("--rotate=+{angle}{page_suffix}")),
    ];
    run_single_output(qpdf, args, &staged, output, cancelled, progress)
}

#[allow(clippy::too_many_arguments)]
fn execute_split<F>(
    qpdf: &Path,
    input: &Path,
    output_directory: &Path,
    pages_per_file: u32,
    job_id: &str,
    cancelled: Arc<AtomicBool>,
    progress: &mut F,
) -> Result<ExecutionOutcome, String>
where
    F: FnMut(u8),
{
    let Some(count) = page_count(qpdf, input, Arc::clone(&cancelled))? else {
        return Ok(ExecutionOutcome::Cancelled);
    };
    let stem = input
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or("Input filename is invalid")?;
    let chunks = count.div_ceil(pages_per_file);
    let mut outputs = Vec::with_capacity(chunks as usize);
    for chunk in 0..chunks {
        let start = chunk * pages_per_file + 1;
        let end = (start + pages_per_file - 1).min(count);
        let name = if start == end {
            format!("{stem}-page-{start}.pdf")
        } else {
            format!("{stem}-pages-{start}-{end}.pdf")
        };
        let final_path = output_directory.join(name);
        if final_path.exists() {
            return Err(format!(
                "Split output already exists: {}",
                final_path.display()
            ));
        }
        let staged = output_directory.join(format!(".{stem}.{job_id}.part-{start}-{end}.pdf"));
        outputs.push((start, end, staged, final_path));
    }

    for (index, (start, end, staged, _)) in outputs.iter().enumerate() {
        let page_spec = if start == end {
            start.to_string()
        } else {
            format!("{start}-{end}")
        };
        let args = vec![
            input.as_os_str().to_owned(),
            OsString::from("--pages"),
            OsString::from("."),
            OsString::from(page_spec),
            OsString::from("--"),
            staged.as_os_str().to_owned(),
        ];
        match run_checked(qpdf, args, Arc::clone(&cancelled)) {
            Ok(Some(())) => validate_pdf_output(qpdf, staged, Arc::clone(&cancelled))?,
            Ok(None) => {
                cleanup_staged(&outputs);
                return Ok(ExecutionOutcome::Cancelled);
            }
            Err(error) => {
                cleanup_staged(&outputs);
                return Err(error);
            }
        }
        progress((((index + 1) * 100) / outputs.len()) as u8);
    }

    let mut finalized = Vec::new();
    for (_, _, staged, final_path) in &outputs {
        if let Err(error) = fs::rename(staged, final_path) {
            for path in finalized {
                let _ = fs::remove_file(path);
            }
            cleanup_staged(&outputs);
            return Err(format!("Split outputs could not be finalized: {error}"));
        }
        finalized.push(final_path);
    }
    progress(100);
    Ok(ExecutionOutcome::Completed)
}

fn run_single_output<F>(
    qpdf: &Path,
    args: Vec<OsString>,
    staged: &Path,
    output: &Path,
    cancelled: Arc<AtomicBool>,
    progress: &mut F,
) -> Result<ExecutionOutcome, String>
where
    F: FnMut(u8),
{
    let _ = fs::remove_file(staged);
    match run_checked(qpdf, args, Arc::clone(&cancelled)) {
        Ok(Some(())) => {}
        Ok(None) => {
            let _ = fs::remove_file(staged);
            return Ok(ExecutionOutcome::Cancelled);
        }
        Err(error) => {
            let _ = fs::remove_file(staged);
            return Err(error);
        }
    }
    if let Err(error) = validate_pdf_output(qpdf, staged, cancelled) {
        let _ = fs::remove_file(staged);
        return Err(error);
    }
    fs::rename(staged, output)
        .map_err(|error| format!("PDF output could not be finalized: {error}"))?;
    progress(100);
    Ok(ExecutionOutcome::Completed)
}

fn page_count(
    qpdf: &Path,
    input: &Path,
    cancelled: Arc<AtomicBool>,
) -> Result<Option<u32>, String> {
    let args = vec![
        OsString::from("--show-npages"),
        input.as_os_str().to_owned(),
    ];
    match run_process(qpdf, args, cancelled)? {
        ProcessOutcome::Cancelled => Ok(None),
        ProcessOutcome::Finished(output) if output.status.success() => output
            .stdout
            .trim()
            .parse::<u32>()
            .map(Some)
            .map_err(|_| "qpdf returned an invalid page count".into()),
        ProcessOutcome::Finished(output) => Err(process_error(output)),
    }
}

fn validate_pdf_output(qpdf: &Path, path: &Path, cancelled: Arc<AtomicBool>) -> Result<(), String> {
    if fs::metadata(path).map_or(true, |metadata| metadata.len() == 0) {
        return Err("qpdf did not produce a valid output file".into());
    }
    let args = vec![OsString::from("--check"), path.as_os_str().to_owned()];
    match run_checked(qpdf, args, cancelled)? {
        Some(()) => Ok(()),
        None => Err("PDF validation was cancelled".into()),
    }
}

fn run_checked(
    qpdf: &Path,
    args: Vec<OsString>,
    cancelled: Arc<AtomicBool>,
) -> Result<Option<()>, String> {
    match run_process(qpdf, args, cancelled)? {
        ProcessOutcome::Cancelled => Ok(None),
        ProcessOutcome::Finished(output)
            if output.status.success() || output.status.code() == Some(3) =>
        {
            Ok(Some(()))
        }
        ProcessOutcome::Finished(output) => Err(process_error(output)),
    }
}

fn run_process(
    qpdf: &Path,
    args: Vec<OsString>,
    cancelled: Arc<AtomicBool>,
) -> Result<ProcessOutcome, String> {
    let mut child = platform::background_command(qpdf)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("qpdf could not be started: {error}"))?;
    let stdout = child.stdout.take().ok_or("qpdf output pipe unavailable")?;
    let stderr = child.stderr.take().ok_or("qpdf error pipe unavailable")?;
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
            Ok(None) => thread::sleep(Duration::from_millis(40)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("qpdf process monitoring failed: {error}"));
            }
        }
    };
    let stdout = stdout_thread.join().unwrap_or_default();
    let stderr = stderr_thread.join().unwrap_or_default();

    Ok(match status {
        Some(status) => ProcessOutcome::Finished(ProcessOutput {
            status,
            stdout,
            stderr,
        }),
        None => ProcessOutcome::Cancelled,
    })
}

fn read_pipe(pipe: impl Read) -> String {
    let mut value = String::new();
    let _ = BufReader::new(pipe).read_to_string(&mut value);
    value
}

fn process_error(output: ProcessOutput) -> String {
    let detail = output.stderr.trim();
    if detail.is_empty() {
        format!("qpdf exited with status {}", output.status)
    } else {
        format!("qpdf operation failed: {detail}")
    }
}

fn validate_pdf(path: &Path) -> Result<(), String> {
    if !path.is_absolute() || !path.is_file() {
        return Err(format!("PDF input does not exist: {}", path.display()));
    }
    if path
        .extension()
        .and_then(|value| value.to_str())
        .is_none_or(|value| !value.eq_ignore_ascii_case("pdf"))
    {
        return Err(format!("Input must be a PDF file: {}", path.display()));
    }
    Ok(())
}

fn validate_output(path: &Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("PDF output path must be absolute".into());
    }
    if path.exists() {
        return Err("The PDF output file already exists".into());
    }
    if path
        .extension()
        .and_then(|value| value.to_str())
        .is_none_or(|value| !value.eq_ignore_ascii_case("pdf"))
    {
        return Err("The PDF output file must use the .pdf extension".into());
    }
    if path.parent().is_none_or(|parent| !parent.is_dir()) {
        return Err("The PDF output directory does not exist".into());
    }
    Ok(())
}

fn validate_page_list(pages: &[u32], allow_empty: bool) -> Result<(), String> {
    if pages.is_empty() {
        return if allow_empty {
            Ok(())
        } else {
            Err("A page order is required".into())
        };
    }
    if pages.contains(&0) {
        return Err("Page numbers start at 1".into());
    }
    let unique: HashSet<_> = pages.iter().collect();
    if unique.len() != pages.len() {
        return Err("Page numbers may not be repeated".into());
    }
    Ok(())
}

fn staged_output_path(output: &Path, job_id: &str) -> Result<PathBuf, String> {
    let parent = output
        .parent()
        .ok_or("PDF output directory is unavailable")?;
    let stem = output
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or("PDF output filename is invalid")?;
    Ok(parent.join(format!(".{stem}.{job_id}.part.pdf")))
}

fn cleanup_staged(outputs: &[(u32, u32, PathBuf, PathBuf)]) {
    for (_, _, staged, _) in outputs {
        let _ = fs::remove_file(staged);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform;
    use tempfile::tempdir;

    fn write_minimal_pdf(path: &Path) {
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources <<>> /Contents 4 0 R >>",
            "<< /Length 0 >>\nstream\n\nendstream",
        ];
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
        }
        let xref = pdf.len();
        pdf.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        for offset in offsets {
            pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
                objects.len() + 1
            )
            .as_bytes(),
        );
        fs::write(path, pdf).unwrap();
    }

    #[test]
    fn merge_requires_two_inputs() {
        let operation = PdfOperation::Merge {
            input_paths: vec!["C:/one.pdf".into()],
            output_path: "C:/merged.pdf".into(),
        };
        assert!(validate_request(&operation)
            .unwrap_err()
            .contains("at least two"));
    }

    #[test]
    fn duplicate_reorder_pages_are_rejected() {
        assert!(validate_page_list(&[1, 2, 2], false)
            .unwrap_err()
            .contains("repeated"));
    }

    #[test]
    fn output_validation_refuses_overwrites() {
        let directory = tempdir().unwrap();
        let output = directory.path().join("output.pdf");
        fs::write(&output, b"existing").unwrap();
        assert!(validate_output(&output)
            .unwrap_err()
            .contains("already exists"));
    }

    #[test]
    #[ignore = "runs the bundled qpdf binary"]
    fn bundled_engine_runs_every_mvp_pdf_operation() {
        let directory = tempdir().unwrap();
        let qpdf = platform::find_qpdf(None).expect("qpdf is required for this smoke test");
        let first = directory.path().join("first.pdf");
        let second = directory.path().join("second.pdf");
        write_minimal_pdf(&first);
        write_minimal_pdf(&second);

        let merged = directory.path().join("merged.pdf");
        let merge = PdfOperation::Merge {
            input_paths: vec![
                first.to_string_lossy().into_owned(),
                second.to_string_lossy().into_owned(),
            ],
            output_path: merged.to_string_lossy().into_owned(),
        };
        assert!(matches!(
            execute(
                &qpdf,
                &merge,
                "merge",
                Arc::new(AtomicBool::new(false)),
                |_| {}
            )
            .unwrap(),
            ExecutionOutcome::Completed
        ));
        assert_eq!(inspect_page_count(&qpdf, &merged).unwrap(), 2);

        let reordered = directory.path().join("reordered.pdf");
        let reorder = PdfOperation::Reorder {
            input_path: merged.to_string_lossy().into_owned(),
            output_path: reordered.to_string_lossy().into_owned(),
            page_order: vec![2, 1],
        };
        assert!(matches!(
            execute(
                &qpdf,
                &reorder,
                "reorder",
                Arc::new(AtomicBool::new(false)),
                |_| {}
            )
            .unwrap(),
            ExecutionOutcome::Completed
        ));

        let rotated = directory.path().join("rotated.pdf");
        let rotate = PdfOperation::Rotate {
            input_path: reordered.to_string_lossy().into_owned(),
            output_path: rotated.to_string_lossy().into_owned(),
            angle: 90,
            pages: vec![1],
        };
        assert!(matches!(
            execute(
                &qpdf,
                &rotate,
                "rotate",
                Arc::new(AtomicBool::new(false)),
                |_| {}
            )
            .unwrap(),
            ExecutionOutcome::Completed
        ));

        let split = PdfOperation::Split {
            input_path: rotated.to_string_lossy().into_owned(),
            output_directory: directory.path().to_string_lossy().into_owned(),
            pages_per_file: 1,
        };
        assert!(matches!(
            execute(
                &qpdf,
                &split,
                "split",
                Arc::new(AtomicBool::new(false)),
                |_| {}
            )
            .unwrap(),
            ExecutionOutcome::Completed
        ));
        assert!(directory.path().join("rotated-page-1.pdf").is_file());
        assert!(directory.path().join("rotated-page-2.pdf").is_file());
    }
}

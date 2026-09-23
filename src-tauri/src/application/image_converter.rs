use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use printpdf::{Mm, Op, PdfDocument, PdfPage, PdfSaveOptions, Pt, RawImage, XObjectTransform};

use crate::{application::pdf_converter, domain::job::ExecutionOutcome};

const SUPPORTED_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "tif", "tiff"];
const PORTRAIT_WIDTH_MM: f32 = 210.0;
const PORTRAIT_HEIGHT_MM: f32 = 297.0;
const PAGE_MARGIN_MM: f32 = 10.0;
const IMAGE_DPI: f32 = 300.0;

pub fn is_supported_input(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| {
            SUPPORTED_EXTENSIONS
                .iter()
                .any(|extension| value.eq_ignore_ascii_case(extension))
        })
}

pub fn validate_request(input_paths: &[String], output_path: &Path) -> Result<(), String> {
    if input_paths.is_empty() {
        return Err("At least one image is required".into());
    }
    for input in input_paths {
        let path = Path::new(input);
        if !path.is_absolute() || !path.is_file() {
            return Err(format!("Image input does not exist: {}", path.display()));
        }
        if !is_supported_input(path) {
            return Err(format!("Unsupported image format: {}", path.display()));
        }
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

pub fn convert<F>(
    qpdf_path: &Path,
    input_paths: &[String],
    output_path: &Path,
    job_id: &str,
    cancelled: Arc<AtomicBool>,
    mut report_progress: F,
) -> Result<ExecutionOutcome, String>
where
    F: FnMut(u8),
{
    validate_request(input_paths, output_path)?;
    let staged = staged_output_path(output_path, job_id)?;
    let _ = fs::remove_file(&staged);
    match build_pdf(
        input_paths,
        &staged,
        Arc::clone(&cancelled),
        &mut report_progress,
    ) {
        Ok(ExecutionOutcome::Completed) => {}
        Ok(ExecutionOutcome::Cancelled) => {
            let _ = fs::remove_file(&staged);
            return Ok(ExecutionOutcome::Cancelled);
        }
        Err(error) => {
            let _ = fs::remove_file(&staged);
            return Err(error);
        }
    }
    if cancelled.load(Ordering::Relaxed) {
        let _ = fs::remove_file(&staged);
        return Ok(ExecutionOutcome::Cancelled);
    }

    let page_count = match pdf_converter::inspect_page_count(qpdf_path, &staged) {
        Ok(count) => count,
        Err(error) => {
            let _ = fs::remove_file(&staged);
            return Err(format!("Generated PDF validation failed: {error}"));
        }
    };
    if page_count as usize != input_paths.len() {
        let _ = fs::remove_file(&staged);
        return Err("Generated PDF contains an unexpected number of pages".into());
    }
    if cancelled.load(Ordering::Relaxed) {
        let _ = fs::remove_file(&staged);
        return Ok(ExecutionOutcome::Cancelled);
    }

    fs::rename(&staged, output_path)
        .map_err(|error| format!("PDF output could not be finalized: {error}"))?;
    report_progress(100);
    Ok(ExecutionOutcome::Completed)
}

fn build_pdf<F>(
    input_paths: &[String],
    staged: &Path,
    cancelled: Arc<AtomicBool>,
    progress: &mut F,
) -> Result<ExecutionOutcome, String>
where
    F: FnMut(u8),
{
    let mut document = PdfDocument::new("Images");
    let mut pages = Vec::with_capacity(input_paths.len());
    let mut warnings = Vec::new();

    for (index, input) in input_paths.iter().enumerate() {
        if cancelled.load(Ordering::Relaxed) {
            return Ok(ExecutionOutcome::Cancelled);
        }
        let bytes = fs::read(input)
            .map_err(|error| format!("Image could not be read ({}): {error}", input))?;
        let image = RawImage::decode_from_bytes(&bytes, &mut warnings)
            .map_err(|error| format!("Image could not be decoded ({}): {error}", input))?;
        if image.width == 0 || image.height == 0 {
            return Err(format!("Image has invalid dimensions: {input}"));
        }

        let landscape = image.width > image.height;
        let (page_width_mm, page_height_mm) = if landscape {
            (PORTRAIT_HEIGHT_MM, PORTRAIT_WIDTH_MM)
        } else {
            (PORTRAIT_WIDTH_MM, PORTRAIT_HEIGHT_MM)
        };
        let available_width = Pt::from(Mm(page_width_mm - PAGE_MARGIN_MM * 2.0)).0;
        let available_height = Pt::from(Mm(page_height_mm - PAGE_MARGIN_MM * 2.0)).0;
        let image_width = image.width as f32 * 72.0 / IMAGE_DPI;
        let image_height = image.height as f32 * 72.0 / IMAGE_DPI;
        let scale = (available_width / image_width)
            .min(available_height / image_height)
            .max(0.000_001);
        let rendered_width = image_width * scale;
        let rendered_height = image_height * scale;
        let page_width = Pt::from(Mm(page_width_mm)).0;
        let page_height = Pt::from(Mm(page_height_mm)).0;
        let image_id = document.add_image(&image);
        pages.push(PdfPage::new(
            Mm(page_width_mm),
            Mm(page_height_mm),
            vec![Op::UseXobject {
                id: image_id,
                transform: XObjectTransform {
                    translate_x: Some(Pt((page_width - rendered_width) / 2.0)),
                    translate_y: Some(Pt((page_height - rendered_height) / 2.0)),
                    scale_x: Some(scale),
                    scale_y: Some(scale),
                    dpi: Some(IMAGE_DPI),
                    ..Default::default()
                },
            }],
        ));
        progress((((index + 1) * 85) / input_paths.len()) as u8);
    }

    if matches!(
        cancellation_outcome(&cancelled),
        ExecutionOutcome::Cancelled
    ) {
        return Ok(ExecutionOutcome::Cancelled);
    }
    let bytes = document
        .with_pages(pages)
        .save(&PdfSaveOptions::default(), &mut warnings);
    fs::write(staged, bytes).map_err(|error| format!("PDF output could not be staged: {error}"))?;
    progress(90);
    Ok(ExecutionOutcome::Completed)
}

fn cancellation_outcome(cancelled: &AtomicBool) -> ExecutionOutcome {
    if cancelled.load(Ordering::Relaxed) {
        ExecutionOutcome::Cancelled
    } else {
        ExecutionOutcome::Completed
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform;
    use tempfile::tempdir;

    const ONE_PIXEL_PNG: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 4,
        0, 0, 0, 181, 28, 12, 2, 0, 0, 0, 11, 73, 68, 65, 84, 120, 218, 99, 100, 248, 15, 0, 1, 5,
        1, 1, 39, 24, 227, 102, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];

    #[test]
    fn empty_image_list_is_rejected() {
        assert!(validate_request(&[], Path::new("C:/output.pdf"))
            .unwrap_err()
            .contains("At least one"));
    }

    #[test]
    fn existing_output_is_rejected() {
        let directory = tempdir().unwrap();
        let input = directory.path().join("image.png");
        let output = directory.path().join("output.pdf");
        fs::write(&input, ONE_PIXEL_PNG).unwrap();
        fs::write(&output, b"existing").unwrap();
        assert!(
            validate_request(&[input.to_string_lossy().into_owned()], &output)
                .unwrap_err()
                .contains("already exists")
        );
    }

    #[test]
    #[ignore = "runs native image encoding and the bundled qpdf binary"]
    fn native_converter_creates_one_page_per_image() {
        let directory = tempdir().unwrap();
        let qpdf = platform::find_qpdf(None).expect("qpdf is required for this smoke test");
        let first = directory.path().join("first.png");
        let second = directory.path().join("second.png");
        let output = directory.path().join("images.pdf");
        fs::write(&first, ONE_PIXEL_PNG).unwrap();
        fs::write(&second, ONE_PIXEL_PNG).unwrap();
        let inputs = vec![
            first.to_string_lossy().into_owned(),
            second.to_string_lossy().into_owned(),
        ];
        assert!(matches!(
            convert(
                &qpdf,
                &inputs,
                &output,
                "test",
                Arc::new(AtomicBool::new(false)),
                |_| {}
            )
            .unwrap(),
            ExecutionOutcome::Completed
        ));
        assert_eq!(
            pdf_converter::inspect_page_count(&qpdf, &output).unwrap(),
            2
        );
    }
}

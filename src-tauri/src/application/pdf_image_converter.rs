use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use image::{DynamicImage, ImageFormat, ImageReader};
use pdfium_render::prelude::{PdfRenderConfig, Pdfium, PdfiumError};

use crate::domain::job::{ExecutionOutcome, PdfImageFormat};

const MAX_RENDER_DIMENSION: i32 = 12_000;

pub fn validate_request(
    input_path: &Path,
    output_directory: &Path,
    _format: PdfImageFormat,
    dpi: u16,
) -> Result<(), String> {
    if !input_path.is_absolute() || !input_path.is_file() {
        return Err(format!(
            "PDF input does not exist: {}",
            input_path.display()
        ));
    }
    if input_path
        .extension()
        .and_then(|value| value.to_str())
        .is_none_or(|value| !value.eq_ignore_ascii_case("pdf"))
    {
        return Err("The input file must use the .pdf extension".into());
    }
    if !output_directory.is_absolute() || !output_directory.is_dir() {
        return Err("The output directory does not exist".into());
    }
    if ![150, 300].contains(&dpi) {
        return Err("PDF rendering DPI must be 150 or 300".into());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn convert<F>(
    pdfium_path: &Path,
    input_path: &Path,
    output_directory: &Path,
    format: PdfImageFormat,
    dpi: u16,
    job_id: &str,
    cancelled: Arc<AtomicBool>,
    mut report_progress: F,
) -> Result<ExecutionOutcome, String>
where
    F: FnMut(u8),
{
    validate_request(input_path, output_directory, format, dpi)?;
    if !pdfium_path.is_file() {
        return Err("The bundled PDFium library could not be found".into());
    }

    let pdfium = bind_pdfium(pdfium_path)?;
    let document = pdfium
        .load_pdf_from_file(input_path, None)
        .map_err(|error| format!("PDFium could not open the PDF: {error}"))?;
    let page_count = document.pages().len() as usize;
    if page_count == 0 {
        return Err("The PDF contains no pages".into());
    }
    let outputs = output_paths(
        input_path,
        output_directory,
        page_count,
        format,
        dpi,
        job_id,
    )?;

    for (index, page) in document.pages().iter().enumerate() {
        if cancelled.load(Ordering::Relaxed) {
            cleanup_staged(&outputs);
            return Ok(ExecutionOutcome::Cancelled);
        }
        let target_width = ((page.width().value * dpi as f32 / 72.0).round() as i32)
            .clamp(1, MAX_RENDER_DIMENSION);
        let config = PdfRenderConfig::new()
            .set_target_width(target_width)
            .set_maximum_height(MAX_RENDER_DIMENSION);
        let bitmap = page
            .render_with_config(&config)
            .map_err(|error| format!("PDF page {} could not be rendered: {error}", index + 1))?;
        let rendered = bitmap
            .as_image()
            .map_err(|error| format!("PDF page {} bitmap is invalid: {error}", index + 1))?;
        let rendered = match format {
            PdfImageFormat::Png => rendered,
            PdfImageFormat::Jpeg => DynamicImage::ImageRgb8(rendered.into_rgb8()),
        };
        let image_format = match format {
            PdfImageFormat::Png => ImageFormat::Png,
            PdfImageFormat::Jpeg => ImageFormat::Jpeg,
        };
        if let Err(error) = rendered.save_with_format(&outputs[index].0, image_format) {
            cleanup_staged(&outputs);
            return Err(format!(
                "PDF page {} could not be saved: {error}",
                index + 1
            ));
        }
        if let Err(error) = validate_image(&outputs[index].0, format) {
            cleanup_staged(&outputs);
            return Err(error);
        }
        report_progress((((index + 1) * 95) / page_count) as u8);
    }

    if cancelled.load(Ordering::Relaxed) {
        cleanup_staged(&outputs);
        return Ok(ExecutionOutcome::Cancelled);
    }
    let mut finalized = Vec::new();
    for (staged, final_path) in &outputs {
        if let Err(error) = fs::rename(staged, final_path) {
            for path in finalized {
                let _ = fs::remove_file(path);
            }
            cleanup_staged(&outputs);
            return Err(format!(
                "Rendered images could not be finalized: {error}. Existing files were preserved; retry to create an incremented filename, choose another output folder, or cancel the job."
            ));
        }
        finalized.push(final_path);
    }
    report_progress(100);
    Ok(ExecutionOutcome::Completed)
}

fn bind_pdfium(path: &Path) -> Result<Pdfium, String> {
    match Pdfium::bind_to_library(path) {
        Ok(bindings) => Ok(Pdfium::new(bindings)),
        Err(PdfiumError::PdfiumLibraryBindingsAlreadyInitialized) => Ok(Pdfium::default()),
        Err(error) => Err(format!("PDFium could not be loaded: {error}")),
    }
}

fn output_paths(
    input_path: &Path,
    output_directory: &Path,
    page_count: usize,
    format: PdfImageFormat,
    dpi: u16,
    job_id: &str,
) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    let stem = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or("PDF input filename is invalid")?;
    let digits = page_count.to_string().len().max(2);
    let format_name = match format {
        PdfImageFormat::Png => "png",
        PdfImageFormat::Jpeg => "jpeg",
    };
    let quality_name = match dpi {
        150 => "normal",
        300 => "high",
        _ => return Err("PDF rendering DPI must be 150 or 300".into()),
    };
    let mut outputs = Vec::with_capacity(page_count);
    for index in 0..page_count {
        let page = index + 1;
        let output_stem = format!(
            "{stem}-page-{page:0digits$}-{format_name}-{quality_name}",
            digits = digits
        );
        let final_path = next_available_path(output_directory, &output_stem, format.extension());
        let staged = output_directory.join(format!(
            ".{stem}.{job_id}.page-{page}.part.{}",
            format.extension()
        ));
        outputs.push((staged, final_path));
    }
    Ok(outputs)
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

fn validate_image(path: &Path, expected_format: PdfImageFormat) -> Result<(), String> {
    let reader = ImageReader::open(path)
        .map_err(|error| format!("Rendered image could not be reopened: {error}"))?
        .with_guessed_format()
        .map_err(|error| format!("Rendered image format could not be detected: {error}"))?;
    let detected = reader.format();
    let expected = match expected_format {
        PdfImageFormat::Png => ImageFormat::Png,
        PdfImageFormat::Jpeg => ImageFormat::Jpeg,
    };
    if detected != Some(expected) {
        return Err("Rendered image has an unexpected format".into());
    }
    let image = reader
        .decode()
        .map_err(|error| format!("Rendered image validation failed: {error}"))?;
    if image.width() == 0 || image.height() == 0 {
        return Err("Rendered image has invalid dimensions".into());
    }
    Ok(())
}

fn cleanup_staged(outputs: &[(PathBuf, PathBuf)]) {
    for (staged, _) in outputs {
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
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 300] /Resources <<>> /Contents 4 0 R >>",
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
    fn unsupported_dpi_is_rejected() {
        let directory = tempdir().unwrap();
        let input = directory.path().join("input.pdf");
        write_minimal_pdf(&input);
        assert!(
            validate_request(&input, directory.path(), PdfImageFormat::Png, 72)
                .unwrap_err()
                .contains("150 or 300")
        );
    }

    #[test]
    fn output_names_include_format_and_quality() {
        let directory = tempdir().unwrap();
        let input = directory.path().join("input.pdf");
        write_minimal_pdf(&input);
        let png = output_paths(
            &input,
            directory.path(),
            1,
            PdfImageFormat::Png,
            150,
            "test",
        )
        .unwrap();
        let jpeg = output_paths(
            &input,
            directory.path(),
            1,
            PdfImageFormat::Jpeg,
            300,
            "test",
        )
        .unwrap();
        assert_eq!(
            png[0].1,
            directory.path().join("input-page-01-png-normal.png")
        );
        assert_eq!(
            jpeg[0].1,
            directory.path().join("input-page-01-jpeg-high.jpg")
        );
    }

    #[test]
    fn output_collisions_receive_incrementing_suffixes() {
        let directory = tempdir().unwrap();
        let input = directory.path().join("input.pdf");
        write_minimal_pdf(&input);
        fs::write(
            directory.path().join("input-page-01-png-normal.png"),
            b"existing",
        )
        .unwrap();
        fs::write(
            directory.path().join("input-page-01-png-normal-2.png"),
            b"existing",
        )
        .unwrap();
        let outputs = output_paths(
            &input,
            directory.path(),
            1,
            PdfImageFormat::Png,
            150,
            "test",
        )
        .unwrap();
        assert_eq!(
            outputs[0].1,
            directory.path().join("input-page-01-png-normal-3.png")
        );
    }

    #[test]
    #[ignore = "loads the bundled PDFium dynamic library"]
    fn bundled_pdfium_renders_png_and_jpeg() {
        let directory = tempdir().unwrap();
        let pdfium = platform::find_pdfium(None).expect("PDFium is required for this smoke test");

        for format in [PdfImageFormat::Png, PdfImageFormat::Jpeg] {
            let source = directory
                .path()
                .join(format!("source-{}.pdf", format.extension()));
            write_minimal_pdf(&source);
            let format_name = if format == PdfImageFormat::Png {
                "png"
            } else {
                "jpeg"
            };
            let first_output = directory.path().join(format!(
                "source-{}-page-01-{format_name}-normal.{}",
                format.extension(),
                format.extension()
            ));
            fs::write(&first_output, b"existing output").unwrap();
            assert!(matches!(
                convert(
                    &pdfium,
                    &source,
                    directory.path(),
                    format,
                    150,
                    format.extension(),
                    Arc::new(AtomicBool::new(false)),
                    |_| {}
                )
                .unwrap(),
                ExecutionOutcome::Completed
            ));
            assert_eq!(fs::read(first_output).unwrap(), b"existing output");
            assert!(directory
                .path()
                .join(format!(
                    "source-{}-page-01-{}-normal-2.{}",
                    format.extension(),
                    format_name,
                    format.extension()
                ))
                .is_file());
        }
    }
}

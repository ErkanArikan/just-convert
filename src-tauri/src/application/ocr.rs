use std::{
    fs,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use image::{DynamicImage, ImageReader};
use kreuzberg_tesseract::TesseractAPI;
use lopdf::Document;
use pdfium_render::prelude::PdfRenderConfig;

use crate::{
    application::{
        ffmpeg, pdf_image_converter,
        searchable_pdf::{self, PageWords},
    },
    domain::job::{ExecutionOutcome, OcrFormat, OcrLanguage},
};

pub const IMAGE_INPUTS: &[&str] = &["png", "jpg", "jpeg", "tif", "tiff", "bmp", "webp"];
pub const SUPPORTED_INPUTS: &[&str] = &["pdf", "png", "jpg", "jpeg", "tif", "tiff", "bmp", "webp"];

const RENDER_DPI: f32 = 300.0;
const MAX_RENDER_DIMENSION: i32 = 12_000;

pub struct OcrRequest<'a> {
    pub tessdata_directory: &'a Path,
    pub pdfium_path: Option<&'a Path>,
    pub input_path: &'a Path,
    pub output_path: &'a Path,
    pub format: OcrFormat,
    pub languages: &'a [OcrLanguage],
    pub job_id: &'a str,
}

struct Page {
    rgb: Vec<u8>,
    width: u32,
    height: u32,
    dpi: i32,
    width_pt: f64,
    height_pt: f64,
}

pub fn is_pdf(path: &Path) -> bool {
    extension(path).as_deref() == Some("pdf")
}

pub fn validate_request(
    input_path: &Path,
    output_path: &Path,
    format: OcrFormat,
    languages: &[OcrLanguage],
) -> Result<(), String> {
    if !input_path.is_absolute() || !output_path.is_absolute() {
        return Err("Input and output paths must be absolute".into());
    }
    if !input_path.is_file() {
        return Err("The selected input file does not exist".into());
    }
    let input_extension = extension(input_path).ok_or("The input file has no extension")?;
    if !SUPPORTED_INPUTS.contains(&input_extension.as_str()) {
        return Err(format!("Unsupported OCR input format: {input_extension}"));
    }
    if languages.is_empty() {
        return Err("Choose at least one recognition language".into());
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

/// Tesseract language string such as `eng+tur`, without duplicates.
pub fn language_string(languages: &[OcrLanguage]) -> String {
    let mut sorted = languages.to_vec();
    sorted.sort();
    sorted.dedup();
    sorted
        .iter()
        .map(|language| language.code())
        .collect::<Vec<_>>()
        .join("+")
}

pub fn convert<F>(
    request: OcrRequest<'_>,
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
        request.languages,
    )?;

    let staged_path = ffmpeg::staged_output_path(request.output_path, request.job_id)?;
    let _ = fs::remove_file(&staged_path);
    let outcome = recognize(&request, &staged_path, &cancelled, &mut report_progress);
    match outcome {
        Ok(ExecutionOutcome::Completed) => {}
        other => {
            let _ = fs::remove_file(&staged_path);
            return other;
        }
    }
    if let Err(error) = validate_output(&staged_path, request.format) {
        let _ = fs::remove_file(&staged_path);
        return Err(error);
    }
    fs::rename(&staged_path, request.output_path)
        .map_err(|error| format!("The recognized output could not be finalized: {error}"))?;
    report_progress(100);
    Ok(ExecutionOutcome::Completed)
}

fn recognize(
    request: &OcrRequest<'_>,
    staged_path: &Path,
    cancelled: &AtomicBool,
    report_progress: &mut impl FnMut(u8),
) -> Result<ExecutionOutcome, String> {
    let api = TesseractAPI::new().map_err(|error| format!("Tesseract could not start: {error}"))?;
    api.init(
        request.tessdata_directory,
        &language_string(request.languages),
    )
    .map_err(|error| format!("Tesseract could not load its language data: {error}"))?;

    let searchable = request.format == OcrFormat::Pdf;
    let mut text = Vec::new();
    let mut layers = Vec::new();
    let mut image_document = None;
    let completed = for_each_page(request, cancelled, |index, count, page| {
        if searchable && !is_pdf(request.input_path) {
            image_document = Some(searchable_pdf::image_document(
                &page.rgb,
                page.width,
                page.height,
                f64::from(page.dpi),
            )?);
        }
        let (page_text, words) = recognize_page(&api, &page, index)?;
        if searchable {
            layers.push(PageWords {
                words,
                image_width: page.width,
                image_height: page.height,
                width_pt: page.width_pt,
                height_pt: page.height_pt,
            });
        } else {
            text.push(page_text);
        }
        report_progress((((index + 1) * 95) / count) as u8);
        Ok(())
    })?;
    if !completed {
        return Ok(ExecutionOutcome::Cancelled);
    }

    if searchable {
        let mut document = match image_document {
            Some(document) => document,
            None => Document::load(request.input_path)
                .map_err(|error| format!("The PDF could not be opened for editing: {error}"))?,
        };
        let font = fs::read(request.tessdata_directory.join("pdf.ttf"))
            .map_err(|error| format!("The PDF font could not be read: {error}"))?;
        searchable_pdf::add_text_layer(&mut document, &layers, &font)?;
        document
            .save(staged_path)
            .map_err(|error| format!("The searchable PDF could not be saved: {error}"))?;
    } else {
        fs::write(staged_path, text.join("\n\n"))
            .map_err(|error| format!("The text could not be written: {error}"))?;
    }
    Ok(ExecutionOutcome::Completed)
}

fn recognize_page(
    api: &TesseractAPI,
    page: &Page,
    index: usize,
) -> Result<(String, Vec<searchable_pdf::Word>), String> {
    let describe = |error: kreuzberg_tesseract::TesseractError| {
        format!("Page {} could not be recognized: {error}", index + 1)
    };
    api.set_image(
        &page.rgb,
        page.width as i32,
        page.height as i32,
        3,
        page.width as i32 * 3,
    )
    .map_err(describe)?;
    api.set_variable("user_defined_dpi", &page.dpi.to_string())
        .map_err(describe)?;
    api.recognize().map_err(describe)?;
    let text = api
        .get_utf8_text()
        .map_err(describe)?
        .trim_end()
        .to_string();
    let words = searchable_pdf::words_from_tsv(&api.get_tsv_text(0).map_err(describe)?);
    Ok((text, words))
}

/// Feeds every page of the input to `handle`; returns false when cancelled.
fn for_each_page<H>(
    request: &OcrRequest<'_>,
    cancelled: &AtomicBool,
    mut handle: H,
) -> Result<bool, String>
where
    H: FnMut(usize, usize, Page) -> Result<(), String>,
{
    if !is_pdf(request.input_path) {
        let decoded = ImageReader::open(request.input_path)
            .and_then(ImageReader::with_guessed_format)
            .map_err(|error| format!("The image could not be read: {error}"))?
            .decode()
            .map_err(|error| format!("The image could not be read: {error}"))?;
        handle(0, 1, page_from_image(decoded, RENDER_DPI as i32, None))?;
        return Ok(true);
    }

    let pdfium_path = request
        .pdfium_path
        .filter(|path| path.is_file())
        .ok_or("The bundled PDFium library is unavailable")?;
    let pdfium = pdf_image_converter::bind_pdfium(pdfium_path)?;
    let document = pdfium
        .load_pdf_from_file(request.input_path, None)
        .map_err(|error| format!("PDFium could not open the PDF: {error}"))?;
    let count = document.pages().len() as usize;
    if count == 0 {
        return Err("The PDF contains no pages".into());
    }
    for (index, page) in document.pages().iter().enumerate() {
        if cancelled.load(Ordering::Relaxed) {
            return Ok(false);
        }
        let width_inches = page.width().value / 72.0;
        let target_width =
            ((width_inches * RENDER_DPI).round() as i32).clamp(1, MAX_RENDER_DIMENSION);
        let config = PdfRenderConfig::new()
            .set_target_width(target_width)
            .set_maximum_height(MAX_RENDER_DIMENSION);
        let rendered = page
            .render_with_config(&config)
            .map_err(|error| format!("PDF page {} could not be rendered: {error}", index + 1))?
            .as_image()
            .map_err(|error| format!("PDF page {} bitmap is invalid: {error}", index + 1))?;
        let dpi = (rendered.width() as f32 / width_inches.max(0.01)).round() as i32;
        let size = (
            f64::from(page.width().value),
            f64::from(page.height().value),
        );
        handle(index, count, page_from_image(rendered, dpi, Some(size)))?;
    }
    Ok(!cancelled.load(Ordering::Relaxed))
}

fn page_from_image(image: DynamicImage, dpi: i32, size_pt: Option<(f64, f64)>) -> Page {
    let rgb = image.into_rgb8();
    let dpi = dpi.max(70);
    let (width, height) = (rgb.width(), rgb.height());
    let (width_pt, height_pt) = size_pt.unwrap_or((
        f64::from(width) * 72.0 / f64::from(dpi),
        f64::from(height) * 72.0 / f64::from(dpi),
    ));
    Page {
        width,
        height,
        rgb: rgb.into_raw(),
        dpi,
        width_pt,
        height_pt,
    }
}

fn validate_output(path: &Path, format: OcrFormat) -> Result<(), String> {
    let metadata = fs::metadata(path).map_err(|_| "No output was produced".to_string())?;
    match format {
        OcrFormat::Pdf => {
            let header =
                fs::read(path).map_err(|error| format!("The output could not be read: {error}"))?;
            if metadata.len() == 0 || !header.starts_with(b"%PDF") {
                return Err("The recognized PDF is not valid".into());
            }
        }
        OcrFormat::Txt => {}
    }
    Ok(())
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn languages_are_sorted_and_deduplicated() {
        assert_eq!(
            language_string(&[
                OcrLanguage::Turkish,
                OcrLanguage::English,
                OcrLanguage::Turkish
            ]),
            "eng+tur"
        );
        assert_eq!(language_string(&[OcrLanguage::Turkish]), "tur");
    }

    #[test]
    fn validation_rejects_bad_requests() {
        let directory = tempdir().unwrap();
        let input = directory.path().join("scan.png");
        let existing = directory.path().join("existing.pdf");
        fs::write(&input, b"x").unwrap();
        fs::write(&existing, b"x").unwrap();
        let english = [OcrLanguage::English];

        assert!(
            validate_request(&input, &existing, OcrFormat::Pdf, &english)
                .unwrap_err()
                .contains("already exists")
        );
        assert!(validate_request(
            &input,
            &directory.path().join("scan.txt"),
            OcrFormat::Pdf,
            &english
        )
        .unwrap_err()
        .contains("must be .pdf"));
        assert!(validate_request(
            &input,
            &directory.path().join("scan.pdf"),
            OcrFormat::Pdf,
            &[]
        )
        .unwrap_err()
        .contains("language"));
        let document = directory.path().join("notes.docx");
        fs::write(&document, b"x").unwrap();
        assert!(validate_request(
            &document,
            &directory.path().join("n.pdf"),
            OcrFormat::Pdf,
            &english
        )
        .unwrap_err()
        .contains("Unsupported"));
        assert!(validate_request(
            &input,
            &directory.path().join("scan.pdf"),
            OcrFormat::Pdf,
            &english
        )
        .is_ok());
    }

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    fn run(
        input: &Path,
        output: &Path,
        format: OcrFormat,
        languages: &[OcrLanguage],
        cancelled: bool,
    ) -> Result<ExecutionOutcome, String> {
        let tessdata = platform::find_tessdata(None).expect("run `pnpm fetch:engines` first");
        let pdfium = platform::find_pdfium(None);
        convert(
            OcrRequest {
                tessdata_directory: &tessdata,
                pdfium_path: pdfium.as_deref(),
                input_path: input,
                output_path: output,
                format,
                languages,
                job_id: "1",
            },
            Arc::new(AtomicBool::new(cancelled)),
            |_| {},
        )
    }

    #[test]
    #[ignore = "needs the fetched Tesseract language data"]
    fn recognizes_english_and_turkish_text_from_images() {
        let directory = tempdir().unwrap();
        let english = directory.path().join("english.txt");
        run(
            &fixture("ocr-en.png"),
            &english,
            OcrFormat::Txt,
            &[OcrLanguage::English],
            false,
        )
        .unwrap();
        let text = fs::read_to_string(&english).unwrap();
        assert!(text.contains("Invoice 2026"), "{text}");
        assert!(text.contains("Quarterly report"), "{text}");

        let turkish = directory.path().join("turkish.txt");
        run(
            &fixture("ocr-tr.png"),
            &turkish,
            OcrFormat::Txt,
            &[OcrLanguage::Turkish],
            false,
        )
        .unwrap();
        let text = fs::read_to_string(&turkish).unwrap();
        assert!(text.contains("güzel"), "{text}");
        assert!(text.contains("teşekkürler"), "{text}");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
    }

    #[test]
    #[ignore = "needs the fetched Tesseract language data and PDFium"]
    fn makes_a_scanned_pdf_searchable() {
        let directory = tempdir().unwrap();
        let output = directory.path().join("searchable.pdf");
        let outcome = run(
            &fixture("ocr-en.pdf"),
            &output,
            OcrFormat::Pdf,
            &[OcrLanguage::English],
            false,
        )
        .unwrap();
        assert!(matches!(outcome, ExecutionOutcome::Completed));

        let pdfium = platform::find_pdfium(None).expect("PDFium is required");
        let pdfium = pdf_image_converter::bind_pdfium(&pdfium).unwrap();
        let document = pdfium.load_pdf_from_file(&output, None).unwrap();
        assert_eq!(document.pages().len(), 1);
        let text = document.pages().get(0).unwrap().text().unwrap().all();
        assert!(text.contains("Invoice 2026"), "{text}");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    #[ignore = "needs the fetched Tesseract language data"]
    fn cancellation_leaves_no_files_behind() {
        let directory = tempdir().unwrap();
        let output = directory.path().join("out.pdf");
        let outcome = run(
            &fixture("ocr-en.pdf"),
            &output,
            OcrFormat::Pdf,
            &[OcrLanguage::English],
            true,
        )
        .unwrap();
        assert!(matches!(outcome, ExecutionOutcome::Cancelled));
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}

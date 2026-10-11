use std::path::Path;

use crate::{
    application::{audio_converter, image_ai, ocr, video_converter},
    domain::converter::{ConverterCapability, ConverterCategory, ConverterStatus},
    platform,
};

pub fn capabilities() -> Vec<ConverterCapability> {
    capabilities_with_dependencies(
        None,
        platform::find_libreoffice(None).is_some(),
        platform::find_pdf_to_docx_runtime(None).is_some(),
    )
}

pub fn capabilities_with_dependencies(
    resource_directory: Option<&Path>,
    libreoffice_ready: bool,
    pdf_to_docx_ready: bool,
) -> Vec<ConverterCapability> {
    let resources = || resource_directory.map(Path::to_path_buf);
    let libreoffice_status = if libreoffice_ready {
        ConverterStatus::Ready
    } else {
        ConverterStatus::ExternalRequired
    };
    let ffmpeg_status = if platform::find_ffmpeg(resources()).is_some() {
        ConverterStatus::Ready
    } else {
        ConverterStatus::ExternalRequired
    };
    let qpdf_status = if platform::find_qpdf(resources()).is_some() {
        ConverterStatus::Ready
    } else {
        ConverterStatus::ExternalRequired
    };
    let pdfium_status = if platform::find_pdfium(resources()).is_some() {
        ConverterStatus::Ready
    } else {
        ConverterStatus::ExternalRequired
    };
    let speech_status = if ffmpeg_status == ConverterStatus::Ready
        && platform::find_whisper_model(resource_directory).is_some()
    {
        ConverterStatus::Ready
    } else {
        ConverterStatus::ExternalRequired
    };
    let ocr_status = if platform::find_tessdata(resources().as_deref()).is_some() {
        ConverterStatus::Ready
    } else {
        ConverterStatus::ExternalRequired
    };
    let image_ai_status = if platform::find_image_ai_models(resources().as_deref()).is_some() {
        ConverterStatus::Ready
    } else {
        ConverterStatus::ExternalRequired
    };
    let tts_status = if platform::find_tts(resources().as_deref()).is_some()
        && ffmpeg_status == ConverterStatus::Ready
    {
        ConverterStatus::Ready
    } else {
        ConverterStatus::ExternalRequired
    };
    let pdf_to_docx_status = if pdf_to_docx_ready {
        ConverterStatus::Ready
    } else {
        ConverterStatus::ExternalRequired
    };

    vec![
        ConverterCapability::new(
            "images-to-pdf",
            ConverterCategory::Image,
            "converters.imagesToPdf.title",
            "converters.imagesToPdf.description",
            "Native",
            qpdf_status.clone(),
            true,
            &["jpg", "jpeg", "png", "webp", "tiff"],
            &["pdf"],
        ),
        ConverterCapability::new(
            "pdf-tools",
            ConverterCategory::Pdf,
            "converters.pdfTools.title",
            "converters.pdfTools.description",
            "qpdf",
            qpdf_status,
            true,
            &["pdf"],
            &["pdf"],
        ),
        ConverterCapability::new(
            "pdf-to-images",
            ConverterCategory::Pdf,
            "converters.pdfToImages.title",
            "converters.pdfToImages.description",
            "PDFium",
            pdfium_status,
            true,
            &["pdf"],
            &["png", "jpg"],
        ),
        ConverterCapability::new(
            "audio",
            ConverterCategory::Audio,
            "converters.audio.title",
            "converters.audio.description",
            "FFmpeg",
            ffmpeg_status.clone(),
            true,
            audio_converter::SUPPORTED_INPUTS,
            &["mp3", "wav", "flac", "aac", "m4a", "ogg"],
        ),
        ConverterCapability::new(
            "remove-background",
            ConverterCategory::Image,
            "converters.removeBackground.title",
            "converters.removeBackground.description",
            "U²-Net",
            image_ai_status.clone(),
            true,
            image_ai::SUPPORTED_INPUTS,
            &["png"],
        ),
        ConverterCapability::new(
            "upscale-image",
            ConverterCategory::Image,
            "converters.upscaleImage.title",
            "converters.upscaleImage.description",
            "Real-ESRGAN",
            image_ai_status,
            true,
            image_ai::SUPPORTED_INPUTS,
            &["png", "jpg"],
        ),
        ConverterCapability::new(
            "video",
            ConverterCategory::Video,
            "converters.video.title",
            "converters.video.description",
            "FFmpeg",
            ffmpeg_status.clone(),
            true,
            video_converter::SUPPORTED_INPUTS,
            &["mp4", "webm", "mov", "mkv", "gif", "png", "jpg"],
        ),
        ConverterCapability::new(
            "ocr",
            ConverterCategory::Document,
            "converters.ocr.title",
            "converters.ocr.description",
            "Tesseract",
            ocr_status,
            true,
            ocr::SUPPORTED_INPUTS,
            &["pdf", "txt"],
        ),
        ConverterCapability::new(
            "text-to-speech",
            ConverterCategory::Audio,
            "converters.textToSpeech.title",
            "converters.textToSpeech.description",
            "Piper voices",
            tts_status,
            true,
            &["txt"],
            &["mp3", "wav", "flac", "aac", "m4a", "ogg"],
        ),
        ConverterCapability::new(
            "speech-to-text",
            ConverterCategory::Audio,
            "converters.speechToText.title",
            "converters.speechToText.description",
            "whisper.cpp",
            speech_status,
            true,
            audio_converter::SUPPORTED_INPUTS,
            &["txt", "srt", "vtt", "json"],
        ),
        ConverterCapability::new(
            "office-to-pdf",
            ConverterCategory::Document,
            "converters.officeToPdf.title",
            "converters.officeToPdf.description",
            "LibreOffice",
            libreoffice_status,
            true,
            &["docx", "xlsx", "pptx"],
            &["pdf"],
        ),
        ConverterCapability::new(
            "pdf-to-docx",
            ConverterCategory::Document,
            "converters.pdfToDocx.title",
            "converters.pdfToDocx.description",
            "PyMuPDF · pdf2docx",
            pdf_to_docx_status,
            true,
            &["pdf"],
            &["docx"],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_contains_every_confirmed_conversion_family() {
        let ids: Vec<_> = capabilities().into_iter().map(|item| item.id).collect();

        for expected in [
            "images-to-pdf",
            "pdf-tools",
            "pdf-to-images",
            "audio",
            "text-to-speech",
            "remove-background",
            "upscale-image",
            "video",
            "ocr",
            "speech-to-text",
            "office-to-pdf",
            "pdf-to-docx",
        ] {
            assert!(ids.contains(&expected.to_string()));
        }
    }
}

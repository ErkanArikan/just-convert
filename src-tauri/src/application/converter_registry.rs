use crate::{
    domain::converter::{ConverterCapability, ConverterCategory, ConverterStatus},
    platform,
};

pub fn capabilities() -> Vec<ConverterCapability> {
    capabilities_with_dependencies(
        platform::find_libreoffice().is_some(),
        platform::find_pdf_to_docx_runtime(None).is_some(),
    )
}

pub fn capabilities_with_dependencies(
    libreoffice_ready: bool,
    pdf_to_docx_ready: bool,
) -> Vec<ConverterCapability> {
    let libreoffice_status = if libreoffice_ready {
        ConverterStatus::Ready
    } else {
        ConverterStatus::ExternalRequired
    };
    let ffmpeg_status = if platform::find_ffmpeg(None).is_some() {
        ConverterStatus::Ready
    } else {
        ConverterStatus::ExternalRequired
    };
    let qpdf_status = if platform::find_qpdf(None).is_some() {
        ConverterStatus::Ready
    } else {
        ConverterStatus::ExternalRequired
    };
    let pdfium_status = if platform::find_pdfium(None).is_some() {
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
            ConverterStatus::Ready,
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
            ffmpeg_status,
            true,
            &["mp3", "wav", "flac", "aac", "m4a", "ogg"],
            &["mp3", "wav", "flac", "aac", "m4a", "ogg"],
        ),
        ConverterCapability::new(
            "office-to-pdf",
            ConverterCategory::Document,
            "converters.officeToPdf.title",
            "converters.officeToPdf.description",
            "LibreOffice",
            libreoffice_status,
            false,
            &["docx", "xlsx", "pptx"],
            &["pdf"],
        ),
        ConverterCapability::new(
            "pdf-to-docx",
            ConverterCategory::Document,
            "converters.pdfToDocx.title",
            "converters.pdfToDocx.description",
            "Python · PyMuPDF · pdf2docx",
            pdf_to_docx_status,
            false,
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
            "office-to-pdf",
            "pdf-to-docx",
        ] {
            assert!(ids.contains(&expected.to_string()));
        }
    }
}

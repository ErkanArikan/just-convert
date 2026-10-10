use rmcp::{
    handler::server::wrapper::Parameters,
    model::{CallToolResult, RequestMetaObject},
    schemars, tool, tool_router, ErrorData, Peer, RoleServer,
};
use serde::Deserialize;
use serde_json::json;

use super::{paths, tool_error, JustConvertServer, WaitOptions};
use crate::{
    application::converter_registry,
    domain::job::{
        AudioFormat, ImageAiOperation, ImageFolderMode, OcrFormat, OcrLanguage, PdfImageFormat,
        PdfOperation, SpeechModel, TranscriptFormat, VideoFormat, VideoQuality,
    },
};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ConvertAudioRequest {
    /// Absolute path of an audio file or a video whose audio track should be extracted.
    pub input_path: String,
    /// Target audio format.
    pub format: AudioFormat,
    /// Absolute output file path with the target extension. Defaults to the input name next to the input.
    pub output_path: Option<String>,
    #[serde(flatten)]
    pub wait: WaitOptions,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ConvertAudioFolderRequest {
    /// Absolute path of a folder; every supported audio file directly inside it is converted.
    pub input_directory: String,
    /// Target audio format.
    pub format: AudioFormat,
    /// Absolute output folder. Defaults to the input folder.
    pub output_directory: Option<String>,
    #[serde(flatten)]
    pub wait: WaitOptions,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ImagesToPdfRequest {
    /// Absolute paths of JPG, PNG, WebP, or TIFF images, in page order.
    pub input_paths: Vec<String>,
    /// Absolute output PDF path. Defaults to the first image's name next to it.
    pub output_path: Option<String>,
    #[serde(flatten)]
    pub wait: WaitOptions,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ImageFolderToPdfRequest {
    /// Absolute path of a folder of images.
    pub input_directory: String,
    /// `combine` makes one PDF from all images; `separate` makes one PDF per image.
    pub mode: ImageFolderMode,
    /// Absolute output folder. Defaults to the input folder.
    pub output_directory: Option<String>,
    #[serde(flatten)]
    pub wait: WaitOptions,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PdfMergeRequest {
    /// Absolute paths of the PDFs to merge, in order.
    pub input_paths: Vec<String>,
    /// Absolute output PDF path. Defaults to `<first>-merged.pdf` next to the first input.
    pub output_path: Option<String>,
    #[serde(flatten)]
    pub wait: WaitOptions,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PdfSplitRequest {
    /// Absolute path of the PDF to split.
    pub input_path: String,
    /// Pages in each output file (1-1000). Use 1 to split into single pages.
    pub pages_per_file: u32,
    /// Absolute output folder. Defaults to the input's folder.
    pub output_directory: Option<String>,
    #[serde(flatten)]
    pub wait: WaitOptions,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PdfReorderRequest {
    /// Absolute path of the PDF.
    pub input_path: String,
    /// New order as 1-based page numbers, listing every page exactly once.
    pub page_order: Vec<u32>,
    /// Absolute output PDF path. Defaults to `<input>-reordered.pdf`.
    pub output_path: Option<String>,
    #[serde(flatten)]
    pub wait: WaitOptions,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PdfRotateRequest {
    /// Absolute path of the PDF.
    pub input_path: String,
    /// Clockwise rotation: 90, 180, or 270.
    pub angle: u16,
    /// 1-based pages to rotate. Empty or omitted rotates every page.
    #[serde(default)]
    pub pages: Vec<u32>,
    /// Absolute output PDF path. Defaults to `<input>-rotated.pdf`.
    pub output_path: Option<String>,
    #[serde(flatten)]
    pub wait: WaitOptions,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PdfPathRequest {
    /// Absolute path of the PDF.
    pub input_path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PdfToImagesRequest {
    /// Absolute path of the PDF.
    pub input_path: String,
    /// Image format for each page.
    pub format: PdfImageFormat,
    /// Render resolution: 150 (normal, default) or 300 (high).
    pub dpi: Option<u16>,
    /// Absolute output folder. Defaults to the input's folder.
    pub output_directory: Option<String>,
    #[serde(flatten)]
    pub wait: WaitOptions,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PdfFolderToImagesRequest {
    /// Absolute path of a folder of PDFs.
    pub input_directory: String,
    /// Image format for each page.
    pub format: PdfImageFormat,
    /// Render resolution: 150 (normal, default) or 300 (high).
    pub dpi: Option<u16>,
    /// Absolute output folder. Defaults to the input folder.
    pub output_directory: Option<String>,
    #[serde(flatten)]
    pub wait: WaitOptions,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FileRequest {
    /// Absolute path of the input file.
    pub input_path: String,
    /// Absolute output file path. Defaults to the input name with the new extension, next to the input.
    pub output_path: Option<String>,
    #[serde(flatten)]
    pub wait: WaitOptions,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FolderRequest {
    /// Absolute path of the input folder.
    pub input_directory: String,
    /// Absolute output folder. Defaults to the input folder.
    pub output_directory: Option<String>,
    #[serde(flatten)]
    pub wait: WaitOptions,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TranscribeRequest {
    /// Absolute path of an audio or video file (MP3, WAV, FLAC, AAC, M4A, OGG, MP4, MOV, MKV, WebM, AVI, M4V).
    pub input_path: String,
    /// Transcript format: plain text, SRT or WebVTT subtitles, or JSON with timestamps. Defaults to txt.
    pub format: Option<TranscriptFormat>,
    /// Spoken language as an ISO 639-1 code such as "en" or "tr". Omit to detect it automatically.
    pub language: Option<String>,
    /// Speech model. Defaults to standard.
    pub model: Option<SpeechModel>,
    /// Absolute output path with the format's extension. Defaults to the input name next to the input.
    pub output_path: Option<String>,
    #[serde(flatten)]
    pub wait: WaitOptions,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ConvertVideoRequest {
    /// Absolute path of a video (MP4, MOV, MKV, WebM, AVI, M4V, MPG, WMV, FLV, 3GP) or animated GIF.
    pub input_path: String,
    /// Target format: mp4 (H.264/AAC), webm (VP9/Opus), mov, mkv, or gif (animated).
    pub format: VideoFormat,
    /// Compression preset. Defaults to balanced; use small to compress, high to stay near the original.
    pub quality: Option<VideoQuality>,
    /// Downscale so the height is at most this many pixels (144-4320); never upscales. Keeps the aspect ratio.
    pub max_height: Option<u32>,
    /// Absolute output path with the format's extension. Defaults to the input name next to the input.
    pub output_path: Option<String>,
    #[serde(flatten)]
    pub wait: WaitOptions,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExtractVideoFramesRequest {
    /// Absolute path of a video file.
    pub input_path: String,
    /// Take one frame every this many seconds (1-3600). Defaults to 1.
    pub interval_seconds: Option<u32>,
    /// Image format of the frames. Defaults to png.
    pub format: Option<PdfImageFormat>,
    /// Absolute output directory. Defaults to the directory containing the video.
    pub output_directory: Option<String>,
    #[serde(flatten)]
    pub wait: WaitOptions,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct OcrRequest {
    /// Absolute path of a scanned PDF or an image (PNG, JPG, TIFF, BMP, WebP).
    pub input_path: String,
    /// pdf makes a searchable PDF with an invisible text layer; txt returns plain text. Defaults to pdf.
    pub format: Option<OcrFormat>,
    /// Languages in the document: eng, tur, or both. Defaults to eng. Choose only tur for Turkish documents; mixing languages lowers accuracy.
    pub languages: Option<Vec<OcrLanguage>>,
    /// Absolute output path with the format's extension. Defaults to the input name next to the input.
    pub output_path: Option<String>,
    #[serde(flatten)]
    pub wait: WaitOptions,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RemoveBackgroundRequest {
    /// Absolute path of a PNG, JPG, WebP, BMP, or TIFF image.
    pub input_path: String,
    /// Absolute output path ending in .png. Defaults to the input name with "-cutout" next to the input.
    pub output_path: Option<String>,
    #[serde(flatten)]
    pub wait: WaitOptions,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpscaleImageRequest {
    /// Absolute path of a PNG, JPG, WebP, BMP, or TIFF image of at most 4 megapixels.
    pub input_path: String,
    /// Enlargement factor, 2 or 4. Defaults to 4.
    pub scale: Option<u8>,
    /// Absolute output path ending in .png or .jpg. Defaults to a PNG named with the factor next to the input.
    pub output_path: Option<String>,
    #[serde(flatten)]
    pub wait: WaitOptions,
}

const DEFAULT_DPI: u16 = 150;

#[tool_router(router = conversion_tools, vis = "pub(super)")]
impl JustConvertServer {
    #[tool(
        description = "List every converter with its engine, supported input/output extensions, and whether it is ready on this machine.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_converters(&self) -> Result<CallToolResult, ErrorData> {
        let capabilities = converter_registry::capabilities_with_dependencies(
            self.dependencies.resource_directory(),
            self.dependencies.libreoffice_path().is_some(),
            self.dependencies.pdf_to_docx_runtime().is_some(),
        );
        let converters: Vec<_> = capabilities
            .into_iter()
            .map(|item| {
                json!({
                    "id": item.id,
                    "category": item.category,
                    "engine": item.engine,
                    "status": item.status,
                    "inputExtensions": item.input_extensions,
                    "outputExtensions": item.output_extensions,
                })
            })
            .collect();
        Ok(CallToolResult::structured(json!({
            "converters": converters,
            "dependencies": self.dependencies.statuses(),
        })))
    }

    #[tool(
        description = "Convert an audio file, or extract the audio track of a video (MP4, MOV, MKV, WebM, AVI, M4V), to MP3, WAV, FLAC, AAC, M4A, or OGG using FFmpeg.",
        annotations(destructive_hint = false, open_world_hint = false)
    )]
    async fn convert_audio(
        &self,
        Parameters(request): Parameters<ConvertAudioRequest>,
        meta: RequestMetaObject,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = (|| {
            let input = paths::absolute(&request.input_path)?;
            let output = paths::output_file(
                request.output_path.as_deref(),
                &input,
                "",
                request.format.extension(),
            )?;
            self.jobs
                .enqueue_audio(input, output, request.format)
                .map(|job| vec![job])
        })();
        Ok(self.finish(result, request.wait, meta, peer).await)
    }

    #[tool(
        description = "Remove the background from a photo, leaving the main subject on a transparent PNG, with a local U²-Net model.",
        annotations(destructive_hint = false, open_world_hint = false)
    )]
    async fn remove_background(
        &self,
        Parameters(request): Parameters<RemoveBackgroundRequest>,
        meta: RequestMetaObject,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = (|| {
            let input = paths::absolute(&request.input_path)?;
            let output =
                paths::output_file(request.output_path.as_deref(), &input, "-cutout", "png")?;
            self.jobs
                .enqueue_image_ai(input, output, ImageAiOperation::RemoveBackground)
                .map(|job| vec![job])
        })();
        Ok(self.finish(result, request.wait, meta, peer).await)
    }

    #[tool(
        description = "Enlarge an image 2x or 4x with the Real-ESRGAN super-resolution model, restoring detail, entirely on this machine.",
        annotations(destructive_hint = false, open_world_hint = false)
    )]
    async fn upscale_image(
        &self,
        Parameters(request): Parameters<UpscaleImageRequest>,
        meta: RequestMetaObject,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = (|| {
            let scale = request.scale.unwrap_or(4);
            let input = paths::absolute(&request.input_path)?;
            let output = paths::output_file(
                request.output_path.as_deref(),
                &input,
                &format!("-{scale}x"),
                "png",
            )?;
            self.jobs
                .enqueue_image_ai(input, output, ImageAiOperation::Upscale { scale })
                .map(|job| vec![job])
        })();
        Ok(self.finish(result, request.wait, meta, peer).await)
    }

    #[tool(
        description = "Recognize text in a scanned PDF or an image with Tesseract (English, Turkish), producing a searchable PDF or a plain text file, entirely on this machine.",
        annotations(destructive_hint = false, open_world_hint = false)
    )]
    async fn ocr(
        &self,
        Parameters(request): Parameters<OcrRequest>,
        meta: RequestMetaObject,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = (|| {
            let format = request.format.unwrap_or(OcrFormat::Pdf);
            let input = paths::absolute(&request.input_path)?;
            let label = if request.output_path.is_none()
                && input.to_lowercase().ends_with(".pdf")
                && format == OcrFormat::Pdf
            {
                "-ocr"
            } else {
                ""
            };
            let output = paths::output_file(
                request.output_path.as_deref(),
                &input,
                label,
                format.extension(),
            )?;
            self.jobs
                .enqueue_ocr(
                    input,
                    output,
                    format,
                    request
                        .languages
                        .clone()
                        .unwrap_or_else(|| vec![OcrLanguage::English]),
                )
                .map(|job| vec![job])
        })();
        Ok(self.finish(result, request.wait, meta, peer).await)
    }

    #[tool(
        description = "Convert or compress a video between MP4, WebM, MOV, MKV, or animated GIF using FFmpeg, with an optional size preset and maximum height.",
        annotations(destructive_hint = false, open_world_hint = false)
    )]
    async fn convert_video(
        &self,
        Parameters(request): Parameters<ConvertVideoRequest>,
        meta: RequestMetaObject,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = (|| {
            let input = paths::absolute(&request.input_path)?;
            let output = paths::output_file(
                request.output_path.as_deref(),
                &input,
                "",
                request.format.extension(),
            )?;
            self.jobs
                .enqueue_video(
                    input,
                    output,
                    request.format,
                    request.quality.unwrap_or_default(),
                    request.max_height,
                )
                .map(|job| vec![job])
        })();
        Ok(self.finish(result, request.wait, meta, peer).await)
    }

    #[tool(
        description = "Extract still frames from a video as PNG or JPEG images, one every N seconds, using FFmpeg.",
        annotations(destructive_hint = false, open_world_hint = false)
    )]
    async fn extract_video_frames(
        &self,
        Parameters(request): Parameters<ExtractVideoFramesRequest>,
        meta: RequestMetaObject,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = (|| {
            let input = paths::absolute(&request.input_path)?;
            let output = paths::output_directory(request.output_directory.as_deref(), &input)?;
            self.jobs
                .enqueue_video_frames(
                    input,
                    output,
                    request.format.unwrap_or(PdfImageFormat::Png),
                    request.interval_seconds.unwrap_or(1),
                )
                .map(|job| vec![job])
        })();
        Ok(self.finish(result, request.wait, meta, peer).await)
    }

    #[tool(
        description = "Transcribe speech from an audio or video file to text or subtitles with Whisper, entirely on this machine.",
        annotations(destructive_hint = false, open_world_hint = false)
    )]
    async fn transcribe(
        &self,
        Parameters(request): Parameters<TranscribeRequest>,
        meta: RequestMetaObject,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = (|| {
            let format = request.format.unwrap_or(TranscriptFormat::Txt);
            let input = paths::absolute(&request.input_path)?;
            let output = paths::output_file(
                request.output_path.as_deref(),
                &input,
                "",
                format.extension(),
            )?;
            self.jobs
                .enqueue_transcription(
                    input,
                    output,
                    format,
                    request.language.clone(),
                    request.model.unwrap_or_default(),
                )
                .map(|job| vec![job])
        })();
        Ok(self.finish(result, request.wait, meta, peer).await)
    }

    #[tool(
        description = "Convert every supported audio file in a folder to one audio format.",
        annotations(destructive_hint = false, open_world_hint = false)
    )]
    async fn convert_audio_folder(
        &self,
        Parameters(request): Parameters<ConvertAudioFolderRequest>,
        meta: RequestMetaObject,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = (|| {
            let input = paths::absolute(&request.input_directory)?;
            let output = paths::output_directory(request.output_directory.as_deref(), &input)?;
            self.jobs
                .enqueue_audio_folder(input, output, request.format)
        })();
        Ok(self.finish(result, request.wait, meta, peer).await)
    }

    #[tool(
        description = "Combine images (JPG, PNG, WebP, TIFF) into a single PDF, one page per image.",
        annotations(destructive_hint = false, open_world_hint = false)
    )]
    async fn images_to_pdf(
        &self,
        Parameters(request): Parameters<ImagesToPdfRequest>,
        meta: RequestMetaObject,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = (|| {
            let inputs = paths::absolute_all(&request.input_paths)?;
            let first = inputs.first().ok_or("At least one image is required")?;
            let output = paths::output_file(request.output_path.as_deref(), first, "", "pdf")?;
            self.jobs
                .enqueue_images_to_pdf(inputs, output)
                .map(|job| vec![job])
        })();
        Ok(self.finish(result, request.wait, meta, peer).await)
    }

    #[tool(
        description = "Turn a folder of images into one combined PDF or one PDF per image.",
        annotations(destructive_hint = false, open_world_hint = false)
    )]
    async fn image_folder_to_pdf(
        &self,
        Parameters(request): Parameters<ImageFolderToPdfRequest>,
        meta: RequestMetaObject,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = (|| {
            let input = paths::absolute(&request.input_directory)?;
            let output = paths::output_directory(request.output_directory.as_deref(), &input)?;
            self.jobs
                .enqueue_image_folder_to_pdf(input, output, request.mode)
        })();
        Ok(self.finish(result, request.wait, meta, peer).await)
    }

    #[tool(
        description = "Merge several PDFs into one, in the given order.",
        annotations(destructive_hint = false, open_world_hint = false)
    )]
    async fn pdf_merge(
        &self,
        Parameters(request): Parameters<PdfMergeRequest>,
        meta: RequestMetaObject,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = (|| {
            let input_paths = paths::absolute_all(&request.input_paths)?;
            let first = input_paths
                .first()
                .ok_or("At least two PDFs are required")?;
            let output_path =
                paths::output_file(request.output_path.as_deref(), first, "-merged", "pdf")?;
            self.jobs
                .enqueue_pdf(PdfOperation::Merge {
                    input_paths,
                    output_path,
                })
                .map(|job| vec![job])
        })();
        Ok(self.finish(result, request.wait, meta, peer).await)
    }

    #[tool(
        description = "Split a PDF into files of N pages each.",
        annotations(destructive_hint = false, open_world_hint = false)
    )]
    async fn pdf_split(
        &self,
        Parameters(request): Parameters<PdfSplitRequest>,
        meta: RequestMetaObject,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = (|| {
            let input_path = paths::absolute(&request.input_path)?;
            let output_directory =
                paths::output_directory(request.output_directory.as_deref(), &input_path)?;
            self.jobs
                .enqueue_pdf(PdfOperation::Split {
                    input_path,
                    output_directory,
                    pages_per_file: request.pages_per_file,
                })
                .map(|job| vec![job])
        })();
        Ok(self.finish(result, request.wait, meta, peer).await)
    }

    #[tool(
        description = "Write a copy of a PDF with its pages in a new order.",
        annotations(destructive_hint = false, open_world_hint = false)
    )]
    async fn pdf_reorder(
        &self,
        Parameters(request): Parameters<PdfReorderRequest>,
        meta: RequestMetaObject,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = (|| {
            let input_path = paths::absolute(&request.input_path)?;
            let output_path = paths::output_file(
                request.output_path.as_deref(),
                &input_path,
                "-reordered",
                "pdf",
            )?;
            self.jobs
                .enqueue_pdf(PdfOperation::Reorder {
                    input_path,
                    output_path,
                    page_order: request.page_order,
                })
                .map(|job| vec![job])
        })();
        Ok(self.finish(result, request.wait, meta, peer).await)
    }

    #[tool(
        description = "Write a copy of a PDF with some or all pages rotated clockwise.",
        annotations(destructive_hint = false, open_world_hint = false)
    )]
    async fn pdf_rotate(
        &self,
        Parameters(request): Parameters<PdfRotateRequest>,
        meta: RequestMetaObject,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = (|| {
            let input_path = paths::absolute(&request.input_path)?;
            let output_path = paths::output_file(
                request.output_path.as_deref(),
                &input_path,
                "-rotated",
                "pdf",
            )?;
            self.jobs
                .enqueue_pdf(PdfOperation::Rotate {
                    input_path,
                    output_path,
                    angle: request.angle,
                    pages: request.pages,
                })
                .map(|job| vec![job])
        })();
        Ok(self.finish(result, request.wait, meta, peer).await)
    }

    #[tool(
        description = "Return the number of pages in a PDF.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn pdf_page_count(
        &self,
        Parameters(request): Parameters<PdfPathRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let result =
            paths::absolute(&request.input_path).and_then(|input| self.jobs.pdf_page_count(&input));
        Ok(match result {
            Ok(pages) => CallToolResult::structured(json!({ "pages": pages })),
            Err(message) => tool_error(message),
        })
    }

    #[tool(
        description = "Render every page of a PDF to PNG or JPEG images using PDFium.",
        annotations(destructive_hint = false, open_world_hint = false)
    )]
    async fn pdf_to_images(
        &self,
        Parameters(request): Parameters<PdfToImagesRequest>,
        meta: RequestMetaObject,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = (|| {
            let input = paths::absolute(&request.input_path)?;
            let output = paths::output_directory(request.output_directory.as_deref(), &input)?;
            self.jobs
                .enqueue_pdf_to_images(
                    input,
                    output,
                    request.format,
                    request.dpi.unwrap_or(DEFAULT_DPI),
                )
                .map(|job| vec![job])
        })();
        Ok(self.finish(result, request.wait, meta, peer).await)
    }

    #[tool(
        description = "Render every page of every PDF in a folder to PNG or JPEG images.",
        annotations(destructive_hint = false, open_world_hint = false)
    )]
    async fn pdf_folder_to_images(
        &self,
        Parameters(request): Parameters<PdfFolderToImagesRequest>,
        meta: RequestMetaObject,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = (|| {
            let input = paths::absolute(&request.input_directory)?;
            let output = paths::output_directory(request.output_directory.as_deref(), &input)?;
            self.jobs.enqueue_pdf_folder_to_images(
                input,
                output,
                request.format,
                request.dpi.unwrap_or(DEFAULT_DPI),
            )
        })();
        Ok(self.finish(result, request.wait, meta, peer).await)
    }

    #[tool(
        description = "Convert a DOCX, XLSX, or PPTX file to PDF using the installed LibreOffice.",
        annotations(destructive_hint = false, open_world_hint = false)
    )]
    async fn office_to_pdf(
        &self,
        Parameters(request): Parameters<FileRequest>,
        meta: RequestMetaObject,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = (|| {
            let input = paths::absolute(&request.input_path)?;
            let output = paths::output_file(request.output_path.as_deref(), &input, "", "pdf")?;
            self.jobs
                .enqueue_office_to_pdf(input, output)
                .map(|job| vec![job])
        })();
        Ok(self.finish(result, request.wait, meta, peer).await)
    }

    #[tool(
        description = "Convert every DOCX, XLSX, and PPTX file in a folder to PDF using LibreOffice.",
        annotations(destructive_hint = false, open_world_hint = false)
    )]
    async fn office_folder_to_pdf(
        &self,
        Parameters(request): Parameters<FolderRequest>,
        meta: RequestMetaObject,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = (|| {
            let input = paths::absolute(&request.input_directory)?;
            let output = paths::output_directory(request.output_directory.as_deref(), &input)?;
            self.jobs.enqueue_office_folder_to_pdf(input, output)
        })();
        Ok(self.finish(result, request.wait, meta, peer).await)
    }

    #[tool(
        description = "Convert a digitally generated (not scanned) PDF to an editable DOCX. Needs Python with pymupdf and pdf2docx.",
        annotations(destructive_hint = false, open_world_hint = false)
    )]
    async fn pdf_to_docx(
        &self,
        Parameters(request): Parameters<FileRequest>,
        meta: RequestMetaObject,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = (|| {
            let input = paths::absolute(&request.input_path)?;
            let output = paths::output_file(request.output_path.as_deref(), &input, "", "docx")?;
            self.jobs
                .enqueue_pdf_to_docx(input, output)
                .map(|job| vec![job])
        })();
        Ok(self.finish(result, request.wait, meta, peer).await)
    }

    #[tool(
        description = "Convert every PDF in a folder to DOCX.",
        annotations(destructive_hint = false, open_world_hint = false)
    )]
    async fn pdf_folder_to_docx(
        &self,
        Parameters(request): Parameters<FolderRequest>,
        meta: RequestMetaObject,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = (|| {
            let input = paths::absolute(&request.input_directory)?;
            let output = paths::output_directory(request.output_directory.as_deref(), &input)?;
            self.jobs.enqueue_pdf_folder_to_docx(input, output)
        })();
        Ok(self.finish(result, request.wait, meta, peer).await)
    }
}

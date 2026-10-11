use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    Mp3,
    Wav,
    Flac,
    Aac,
    M4a,
    Ogg,
}

#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PdfImageFormat {
    Png,
    Jpeg,
}

#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImageFolderMode {
    Combine,
    Separate,
}

impl PdfImageFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
        }
    }
}

impl AudioFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Wav => "wav",
            Self::Flac => "flac",
            Self::Aac => "aac",
            Self::M4a => "m4a",
            Self::Ogg => "ogg",
        }
    }
}

#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TranscriptFormat {
    Txt,
    Srt,
    Vtt,
    Json,
}

impl TranscriptFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Txt => "txt",
            Self::Srt => "srt",
            Self::Vtt => "vtt",
            Self::Json => "json",
        }
    }
}

#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeechModel {
    /// Bundled multilingual Whisper base model; works offline.
    #[default]
    Standard,
    /// Whisper large-v3-turbo; downloaded once on first use.
    HighAccuracy,
}

#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VideoFormat {
    Mp4,
    Webm,
    Mov,
    Mkv,
    Gif,
}

impl VideoFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Webm => "webm",
            Self::Mov => "mov",
            Self::Mkv => "mkv",
            Self::Gif => "gif",
        }
    }
}

#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OcrFormat {
    /// Searchable PDF: the page images with an invisible text layer.
    Pdf,
    Txt,
}

impl OcrFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Txt => "txt",
        }
    }
}

#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum OcrLanguage {
    #[serde(rename = "eng")]
    English,
    #[serde(rename = "tur")]
    Turkish,
}

impl OcrLanguage {
    pub fn code(self) -> &'static str {
        match self {
            Self::English => "eng",
            Self::Turkish => "tur",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageAiOperation {
    /// Cuts out the main subject, leaving a transparent background.
    RemoveBackground,
    /// Enlarges the image 2x or 4x with a super-resolution network.
    Upscale { scale: u8 },
}

#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TtsVoice {
    /// American English, female (Piper en_US-ljspeech).
    English,
    /// Turkish, male (Piper tr_TR-fettah).
    Turkish,
}

impl TtsVoice {
    pub fn directory(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Turkish => "tr",
        }
    }
}

#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TtsSpeed {
    Slow,
    #[default]
    Normal,
    Fast,
}

impl TtsSpeed {
    /// The VITS length scale: larger values speak more slowly.
    pub fn length_scale(self) -> &'static str {
        match self {
            Self::Slow => "1.25",
            Self::Normal => "1.0",
            Self::Fast => "0.8",
        }
    }
}

/// Compression preset: trades file size against visual quality.
#[cfg_attr(feature = "mcp", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VideoQuality {
    /// Near-original quality, largest files.
    High,
    #[default]
    Balanced,
    /// Smallest files, visible compression.
    Small,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum PdfOperation {
    Merge {
        input_paths: Vec<String>,
        output_path: String,
    },
    Split {
        input_path: String,
        output_directory: String,
        pages_per_file: u32,
    },
    Reorder {
        input_path: String,
        output_path: String,
        page_order: Vec<u32>,
    },
    Rotate {
        input_path: String,
        output_path: String,
        angle: u16,
        pages: Vec<u32>,
    },
}

#[derive(Debug)]
pub enum ExecutionOutcome {
    Completed,
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JobKind {
    AudioConversion {
        input_path: String,
        output_path: String,
        format: AudioFormat,
    },
    PdfOperation {
        operation: PdfOperation,
    },
    ImagesToPdf {
        input_paths: Vec<String>,
        output_path: String,
    },
    PdfToImages {
        input_path: String,
        output_directory: String,
        format: PdfImageFormat,
        dpi: u16,
    },
    OfficeToPdf {
        input_path: String,
        output_path: String,
    },
    PdfToDocx {
        input_path: String,
        output_path: String,
    },
    Transcription {
        input_path: String,
        output_path: String,
        format: TranscriptFormat,
        language: Option<String>,
        model: SpeechModel,
    },
    VideoConversion {
        input_path: String,
        output_path: String,
        format: VideoFormat,
        quality: VideoQuality,
        max_height: Option<u32>,
    },
    VideoFrames {
        input_path: String,
        output_directory: String,
        format: PdfImageFormat,
        interval_seconds: u32,
    },
    Ocr {
        input_path: String,
        output_path: String,
        format: OcrFormat,
        languages: Vec<OcrLanguage>,
    },
    ImageAi {
        input_path: String,
        output_path: String,
        operation: ImageAiOperation,
    },
    TextToSpeech {
        text: String,
        output_path: String,
        format: AudioFormat,
        voice: TtsVoice,
        speed: TtsSpeed,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobError {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub job_type: String,
    pub kind: JobKind,
    pub status: JobStatus,
    pub progress: u8,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
    pub error: Option<JobError>,
}

impl Job {
    pub fn queued_audio(
        id: impl Into<String>,
        input_path: impl Into<String>,
        output_path: impl Into<String>,
        format: AudioFormat,
        created_at: u64,
    ) -> Self {
        Self {
            id: id.into(),
            job_type: "audio_conversion".into(),
            kind: JobKind::AudioConversion {
                input_path: input_path.into(),
                output_path: output_path.into(),
                format,
            },
            status: JobStatus::Queued,
            progress: 0,
            created_at,
            started_at: None,
            finished_at: None,
            error: None,
        }
    }

    pub fn queued_pdf(id: impl Into<String>, operation: PdfOperation, created_at: u64) -> Self {
        Self {
            id: id.into(),
            job_type: "pdf_operation".into(),
            kind: JobKind::PdfOperation { operation },
            status: JobStatus::Queued,
            progress: 0,
            created_at,
            started_at: None,
            finished_at: None,
            error: None,
        }
    }

    pub fn queued_images_to_pdf(
        id: impl Into<String>,
        input_paths: Vec<String>,
        output_path: impl Into<String>,
        created_at: u64,
    ) -> Self {
        Self {
            id: id.into(),
            job_type: "images_to_pdf".into(),
            kind: JobKind::ImagesToPdf {
                input_paths,
                output_path: output_path.into(),
            },
            status: JobStatus::Queued,
            progress: 0,
            created_at,
            started_at: None,
            finished_at: None,
            error: None,
        }
    }

    pub fn queued_pdf_to_images(
        id: impl Into<String>,
        input_path: impl Into<String>,
        output_directory: impl Into<String>,
        format: PdfImageFormat,
        dpi: u16,
        created_at: u64,
    ) -> Self {
        Self {
            id: id.into(),
            job_type: "pdf_to_images".into(),
            kind: JobKind::PdfToImages {
                input_path: input_path.into(),
                output_directory: output_directory.into(),
                format,
                dpi,
            },
            status: JobStatus::Queued,
            progress: 0,
            created_at,
            started_at: None,
            finished_at: None,
            error: None,
        }
    }

    pub fn queued_office_to_pdf(
        id: impl Into<String>,
        input_path: impl Into<String>,
        output_path: impl Into<String>,
        created_at: u64,
    ) -> Self {
        Self {
            id: id.into(),
            job_type: "office_to_pdf".into(),
            kind: JobKind::OfficeToPdf {
                input_path: input_path.into(),
                output_path: output_path.into(),
            },
            status: JobStatus::Queued,
            progress: 0,
            created_at,
            started_at: None,
            finished_at: None,
            error: None,
        }
    }

    pub fn queued_pdf_to_docx(
        id: impl Into<String>,
        input_path: impl Into<String>,
        output_path: impl Into<String>,
        created_at: u64,
    ) -> Self {
        Self {
            id: id.into(),
            job_type: "pdf_to_docx".into(),
            kind: JobKind::PdfToDocx {
                input_path: input_path.into(),
                output_path: output_path.into(),
            },
            status: JobStatus::Queued,
            progress: 0,
            created_at,
            started_at: None,
            finished_at: None,
            error: None,
        }
    }

    pub fn queued_transcription(
        id: impl Into<String>,
        input_path: impl Into<String>,
        output_path: impl Into<String>,
        format: TranscriptFormat,
        language: Option<String>,
        model: SpeechModel,
        created_at: u64,
    ) -> Self {
        Self {
            id: id.into(),
            job_type: "transcription".into(),
            kind: JobKind::Transcription {
                input_path: input_path.into(),
                output_path: output_path.into(),
                format,
                language,
                model,
            },
            status: JobStatus::Queued,
            progress: 0,
            created_at,
            started_at: None,
            finished_at: None,
            error: None,
        }
    }

    pub fn queued_video_conversion(
        id: impl Into<String>,
        input_path: impl Into<String>,
        output_path: impl Into<String>,
        format: VideoFormat,
        quality: VideoQuality,
        max_height: Option<u32>,
        created_at: u64,
    ) -> Self {
        Self {
            id: id.into(),
            job_type: "video_conversion".into(),
            kind: JobKind::VideoConversion {
                input_path: input_path.into(),
                output_path: output_path.into(),
                format,
                quality,
                max_height,
            },
            status: JobStatus::Queued,
            progress: 0,
            created_at,
            started_at: None,
            finished_at: None,
            error: None,
        }
    }

    pub fn queued_video_frames(
        id: impl Into<String>,
        input_path: impl Into<String>,
        output_directory: impl Into<String>,
        format: PdfImageFormat,
        interval_seconds: u32,
        created_at: u64,
    ) -> Self {
        Self {
            id: id.into(),
            job_type: "video_frames".into(),
            kind: JobKind::VideoFrames {
                input_path: input_path.into(),
                output_directory: output_directory.into(),
                format,
                interval_seconds,
            },
            status: JobStatus::Queued,
            progress: 0,
            created_at,
            started_at: None,
            finished_at: None,
            error: None,
        }
    }

    pub fn queued_ocr(
        id: impl Into<String>,
        input_path: impl Into<String>,
        output_path: impl Into<String>,
        format: OcrFormat,
        languages: Vec<OcrLanguage>,
        created_at: u64,
    ) -> Self {
        Self {
            id: id.into(),
            job_type: "ocr".into(),
            kind: JobKind::Ocr {
                input_path: input_path.into(),
                output_path: output_path.into(),
                format,
                languages,
            },
            status: JobStatus::Queued,
            progress: 0,
            created_at,
            started_at: None,
            finished_at: None,
            error: None,
        }
    }

    pub fn queued_image_ai(
        id: impl Into<String>,
        input_path: impl Into<String>,
        output_path: impl Into<String>,
        operation: ImageAiOperation,
        created_at: u64,
    ) -> Self {
        Self {
            id: id.into(),
            job_type: "image_ai".into(),
            kind: JobKind::ImageAi {
                input_path: input_path.into(),
                output_path: output_path.into(),
                operation,
            },
            status: JobStatus::Queued,
            progress: 0,
            created_at,
            started_at: None,
            finished_at: None,
            error: None,
        }
    }

    pub fn queued_text_to_speech(
        id: impl Into<String>,
        text: impl Into<String>,
        output_path: impl Into<String>,
        format: AudioFormat,
        voice: TtsVoice,
        speed: TtsSpeed,
        created_at: u64,
    ) -> Self {
        Self {
            id: id.into(),
            job_type: "text_to_speech".into(),
            kind: JobKind::TextToSpeech {
                text: text.into(),
                output_path: output_path.into(),
                format,
                voice,
                speed,
            },
            status: JobStatus::Queued,
            progress: 0,
            created_at,
            started_at: None,
            finished_at: None,
            error: None,
        }
    }
}

use serde::{Deserialize, Serialize};

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

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PdfImageFormat {
    Png,
    Jpeg,
}

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
}

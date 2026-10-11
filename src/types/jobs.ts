export type AudioFormat = "mp3" | "wav" | "flac" | "aac" | "m4a" | "ogg";
export type JobStatus =
  "queued" | "running" | "completed" | "failed" | "cancelled";

export interface JobError {
  code: string;
  message: string;
}

export interface AudioJobKind {
  type: "audio_conversion";
  input_path: string;
  output_path: string;
  format: AudioFormat;
}

export type PdfOperation =
  | { type: "merge"; inputPaths: string[]; outputPath: string }
  | {
      type: "split";
      inputPath: string;
      outputDirectory: string;
      pagesPerFile: number;
    }
  | {
      type: "reorder";
      inputPath: string;
      outputPath: string;
      pageOrder: number[];
    }
  | {
      type: "rotate";
      inputPath: string;
      outputPath: string;
      angle: number;
      pages: number[];
    };

export interface PdfJobKind {
  type: "pdf_operation";
  operation: PdfOperation;
}

export interface ImagesToPdfJobKind {
  type: "images_to_pdf";
  input_paths: string[];
  output_path: string;
}

export interface PdfToImagesJobKind {
  type: "pdf_to_images";
  input_path: string;
  output_directory: string;
  format: "png" | "jpeg";
  dpi: 150 | 300;
}

export interface OfficeToPdfJobKind {
  type: "office_to_pdf";
  input_path: string;
  output_path: string;
}

export interface PdfToDocxJobKind {
  type: "pdf_to_docx";
  input_path: string;
  output_path: string;
}

export type TranscriptFormat = "txt" | "srt" | "vtt" | "json";
export type SpeechModel = "standard" | "high_accuracy";

export interface TranscriptionJobKind {
  type: "transcription";
  input_path: string;
  output_path: string;
  format: TranscriptFormat;
  language: string | null;
  model: SpeechModel;
}

export type VideoFormat = "mp4" | "webm" | "mov" | "mkv" | "gif";
export type VideoQuality = "high" | "balanced" | "small";

export interface VideoConversionJobKind {
  type: "video_conversion";
  input_path: string;
  output_path: string;
  format: VideoFormat;
  quality: VideoQuality;
  max_height: number | null;
}

export interface VideoFramesJobKind {
  type: "video_frames";
  input_path: string;
  output_directory: string;
  format: "png" | "jpeg";
  interval_seconds: number;
}

export type OcrFormat = "pdf" | "txt";
export type OcrLanguage = "eng" | "tur";

export interface OcrJobKind {
  type: "ocr";
  input_path: string;
  output_path: string;
  format: OcrFormat;
  languages: OcrLanguage[];
}

export type ImageAiOperation =
  { type: "remove_background" } | { type: "upscale"; scale: 2 | 4 };

export interface ImageAiJobKind {
  type: "image_ai";
  input_path: string;
  output_path: string;
  operation: ImageAiOperation;
}

export type TtsVoice = "english" | "turkish";
export type TtsSpeed = "slow" | "normal" | "fast";

export interface TextToSpeechJobKind {
  type: "text_to_speech";
  text: string;
  output_path: string;
  format: AudioFormat;
  voice: TtsVoice;
  speed: TtsSpeed;
}

export interface ConversionJob {
  id: string;
  jobType:
    | "audio_conversion"
    | "pdf_operation"
    | "images_to_pdf"
    | "pdf_to_images"
    | "office_to_pdf"
    | "pdf_to_docx"
    | "transcription"
    | "video_conversion"
    | "video_frames"
    | "ocr"
    | "image_ai"
    | "text_to_speech";
  kind:
    | AudioJobKind
    | PdfJobKind
    | ImagesToPdfJobKind
    | PdfToImagesJobKind
    | OfficeToPdfJobKind
    | PdfToDocxJobKind
    | TranscriptionJobKind
    | VideoConversionJobKind
    | VideoFramesJobKind
    | OcrJobKind
    | ImageAiJobKind
    | TextToSpeechJobKind;
  status: JobStatus;
  progress: number;
  createdAt: number;
  startedAt: number | null;
  finishedAt: number | null;
  error: JobError | null;
}

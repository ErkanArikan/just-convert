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

export interface ConversionJob {
  id: string;
  jobType:
    | "audio_conversion"
    | "pdf_operation"
    | "images_to_pdf"
    | "pdf_to_images"
    | "office_to_pdf"
    | "pdf_to_docx";
  kind:
    | AudioJobKind
    | PdfJobKind
    | ImagesToPdfJobKind
    | PdfToImagesJobKind
    | OfficeToPdfJobKind
    | PdfToDocxJobKind;
  status: JobStatus;
  progress: number;
  createdAt: number;
  startedAt: number | null;
  finishedAt: number | null;
  error: JobError | null;
}

import { invoke, isTauri } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import type {
  AppBootstrap,
  DependencyId,
  DependencyStatuses,
} from "../types/bootstrap";
import type {
  DirectoryListing,
  FileOperationResult,
} from "../types/fileManager";
import type { AudioFormat, ConversionJob, PdfOperation } from "../types/jobs";

const browserFallback: AppBootstrap = {
  appVersion: "0.1.0-dev",
  platform: "browser-preview",
  converters: [
    {
      id: "images-to-pdf",
      category: "image",
      titleKey: "converters.imagesToPdf.title",
      descriptionKey: "converters.imagesToPdf.description",
      engine: "Native",
      status: "ready",
      bundled: true,
      inputExtensions: ["jpg", "jpeg", "png", "webp", "tiff"],
      outputExtensions: ["pdf"],
    },
    {
      id: "pdf-tools",
      category: "pdf",
      titleKey: "converters.pdfTools.title",
      descriptionKey: "converters.pdfTools.description",
      engine: "qpdf",
      status: "ready",
      bundled: true,
      inputExtensions: ["pdf"],
      outputExtensions: ["pdf"],
    },
    {
      id: "pdf-to-images",
      category: "pdf",
      titleKey: "converters.pdfToImages.title",
      descriptionKey: "converters.pdfToImages.description",
      engine: "PDFium",
      status: "ready",
      bundled: true,
      inputExtensions: ["pdf"],
      outputExtensions: ["png", "jpg"],
    },
    {
      id: "audio",
      category: "audio",
      titleKey: "converters.audio.title",
      descriptionKey: "converters.audio.description",
      engine: "FFmpeg",
      status: "ready",
      bundled: true,
      inputExtensions: ["mp3", "wav", "flac", "aac", "m4a", "ogg"],
      outputExtensions: ["mp3", "wav", "flac", "aac", "m4a", "ogg"],
    },
    {
      id: "office-to-pdf",
      category: "document",
      titleKey: "converters.officeToPdf.title",
      descriptionKey: "converters.officeToPdf.description",
      engine: "LibreOffice",
      status: "external_required",
      bundled: false,
      inputExtensions: ["docx", "xlsx", "pptx"],
      outputExtensions: ["pdf"],
    },
    {
      id: "pdf-to-docx",
      category: "document",
      titleKey: "converters.pdfToDocx.title",
      descriptionKey: "converters.pdfToDocx.description",
      engine: "Python · PyMuPDF · pdf2docx",
      status: "external_required",
      bundled: false,
      inputExtensions: ["pdf"],
      outputExtensions: ["docx"],
    },
  ],
  dependencies: {
    libreoffice: {
      id: "libreoffice",
      automaticPath: null,
      manualPath: null,
      manualPathValid: false,
      resolvedPath: null,
      ready: false,
      issue: null,
    },
    python: {
      id: "python",
      automaticPath: null,
      manualPath: null,
      manualPathValid: false,
      resolvedPath: null,
      ready: false,
      issue: null,
    },
  },
};

export async function getAppBootstrap(): Promise<AppBootstrap> {
  if (!isTauri()) return browserFallback;
  return invoke<AppBootstrap>("get_app_bootstrap");
}

export async function openExternalUrl(url: string): Promise<void> {
  if (!isTauri()) {
    window.open(url, "_blank", "noopener,noreferrer");
    return;
  }
  await openUrl(url);
}

export async function chooseDependencyExecutable(
  dependency: DependencyId,
): Promise<string | null> {
  requireDesktop();
  const selected = await open({
    directory: false,
    multiple: false,
    filters: [
      {
        name: dependency === "libreoffice" ? "soffice.exe" : "python.exe",
        extensions: ["exe"],
      },
    ],
  });
  return typeof selected === "string" ? selected : null;
}

export async function setDependencyOverride(
  dependency: DependencyId,
  path: string | null,
): Promise<DependencyStatuses> {
  requireDesktop();
  return invoke<DependencyStatuses>("set_dependency_override", {
    dependency,
    path,
  });
}

function requireDesktop() {
  if (!isTauri())
    throw new Error(
      "File operations are available in the desktop application.",
    );
}

export async function chooseDirectory(): Promise<string | null> {
  requireDesktop();
  const selected = await open({ directory: true, multiple: false });
  return typeof selected === "string" ? selected : null;
}

export async function chooseAudioFile(): Promise<string | null> {
  requireDesktop();
  const selected = await open({
    directory: false,
    multiple: false,
    filters: [
      {
        name: "Audio",
        extensions: ["mp3", "wav", "flac", "aac", "m4a", "ogg"],
      },
    ],
  });
  return typeof selected === "string" ? selected : null;
}

export async function chooseOfficeFile(): Promise<string | null> {
  requireDesktop();
  const selected = await open({
    directory: false,
    multiple: false,
    filters: [
      { name: "Office documents", extensions: ["docx", "xlsx", "pptx"] },
    ],
  });
  return typeof selected === "string" ? selected : null;
}

export async function chooseDocxOutput(
  defaultPath: string,
): Promise<string | null> {
  requireDesktop();
  const selected = await save({
    defaultPath,
    filters: [{ name: "Word document", extensions: ["docx"] }],
  });
  return typeof selected === "string" ? selected : null;
}

export async function chooseAudioOutput(
  defaultPath: string,
  format: AudioFormat,
): Promise<string | null> {
  requireDesktop();
  const selected = await save({
    defaultPath,
    filters: [{ name: format.toUpperCase(), extensions: [format] }],
  });
  return typeof selected === "string" ? selected : null;
}

export async function choosePdfFiles(multiple: boolean): Promise<string[]> {
  requireDesktop();
  const selected = await open({
    directory: false,
    multiple,
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  if (Array.isArray(selected)) return selected;
  return typeof selected === "string" ? [selected] : [];
}

export async function choosePdfOutput(
  defaultPath: string,
): Promise<string | null> {
  requireDesktop();
  const selected = await save({
    defaultPath,
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  return typeof selected === "string" ? selected : null;
}

export async function chooseImageFiles(): Promise<string[]> {
  requireDesktop();
  const selected = await open({
    directory: false,
    multiple: true,
    filters: [
      {
        name: "Images",
        extensions: ["jpg", "jpeg", "png", "webp", "tif", "tiff"],
      },
    ],
  });
  if (Array.isArray(selected)) return selected;
  return typeof selected === "string" ? [selected] : [];
}

export async function enqueueImagesToPdf(
  inputPaths: string[],
  outputPath: string,
): Promise<ConversionJob> {
  requireDesktop();
  return invoke<ConversionJob>("enqueue_images_to_pdf", {
    inputPaths,
    outputPath,
  });
}

export async function enqueueImageFolderToPdf(
  inputDirectory: string,
  outputDirectory: string,
  mode: "combine" | "separate",
): Promise<ConversionJob[]> {
  requireDesktop();
  return invoke<ConversionJob[]>("enqueue_image_folder_to_pdf", {
    inputDirectory,
    outputDirectory,
    mode,
  });
}

export async function enqueuePdfToImages(
  inputPath: string,
  outputDirectory: string,
  format: "png" | "jpeg",
  dpi: 150 | 300,
): Promise<ConversionJob> {
  requireDesktop();
  return invoke<ConversionJob>("enqueue_pdf_to_images", {
    inputPath,
    outputDirectory,
    format,
    dpi,
  });
}

export async function enqueuePdfFolderToImages(
  inputDirectory: string,
  outputDirectory: string,
  format: "png" | "jpeg",
  dpi: 150 | 300,
): Promise<ConversionJob[]> {
  requireDesktop();
  return invoke<ConversionJob[]>("enqueue_pdf_folder_to_images", {
    inputDirectory,
    outputDirectory,
    format,
    dpi,
  });
}

export async function enqueueOfficeToPdf(
  inputPath: string,
  outputPath: string,
): Promise<ConversionJob> {
  requireDesktop();
  return invoke<ConversionJob>("enqueue_office_to_pdf", {
    inputPath,
    outputPath,
  });
}

export async function enqueueOfficeFolderToPdf(
  inputDirectory: string,
  outputDirectory: string,
): Promise<ConversionJob[]> {
  requireDesktop();
  return invoke<ConversionJob[]>("enqueue_office_folder_to_pdf", {
    inputDirectory,
    outputDirectory,
  });
}

export async function enqueuePdfToDocx(
  inputPath: string,
  outputPath: string,
): Promise<ConversionJob> {
  requireDesktop();
  return invoke<ConversionJob>("enqueue_pdf_to_docx", {
    inputPath,
    outputPath,
  });
}

export async function enqueuePdfFolderToDocx(
  inputDirectory: string,
  outputDirectory: string,
): Promise<ConversionJob[]> {
  requireDesktop();
  return invoke<ConversionJob[]>("enqueue_pdf_folder_to_docx", {
    inputDirectory,
    outputDirectory,
  });
}

export async function enqueueAudioConversion(
  inputPath: string,
  outputPath: string,
  format: AudioFormat,
): Promise<ConversionJob> {
  requireDesktop();
  return invoke<ConversionJob>("enqueue_audio_conversion", {
    inputPath,
    outputPath,
    format,
  });
}

export async function enqueueAudioFolderConversion(
  inputDirectory: string,
  outputDirectory: string,
  format: AudioFormat,
): Promise<ConversionJob[]> {
  requireDesktop();
  return invoke<ConversionJob[]>("enqueue_audio_folder_conversion", {
    inputDirectory,
    outputDirectory,
    format,
  });
}

export async function listJobs(): Promise<ConversionJob[]> {
  if (!isTauri()) return [];
  return invoke<ConversionJob[]>("list_jobs");
}

export async function enqueuePdfOperation(
  operation: PdfOperation,
): Promise<ConversionJob> {
  requireDesktop();
  return invoke<ConversionJob>("enqueue_pdf_operation", { operation });
}

export async function getPdfPageCount(inputPath: string): Promise<number> {
  requireDesktop();
  return invoke<number>("get_pdf_page_count", { inputPath });
}

export async function cancelJob(id: string): Promise<ConversionJob> {
  requireDesktop();
  return invoke<ConversionJob>("cancel_job", { id });
}

export async function deleteJobHistory(id: string): Promise<void> {
  requireDesktop();
  return invoke<void>("delete_job_history", { id });
}

export async function clearJobHistory(): Promise<number> {
  requireDesktop();
  return invoke<number>("clear_job_history");
}

export async function clearJobHistorySection(
  section: "completed" | "failed",
): Promise<number> {
  requireDesktop();
  return invoke<number>("clear_job_history_section", { section });
}

export async function listDirectory(path: string): Promise<DirectoryListing> {
  requireDesktop();
  return invoke<DirectoryListing>("list_directory", { path });
}

export async function createDirectory(
  parentPath: string,
  name: string,
): Promise<FileOperationResult> {
  requireDesktop();
  return invoke<FileOperationResult>("create_directory", { parentPath, name });
}

export async function renameEntry(
  path: string,
  newName: string,
): Promise<FileOperationResult> {
  requireDesktop();
  return invoke<FileOperationResult>("rename_entry", { path, newName });
}

export async function copyEntries(
  sources: string[],
  destination: string,
): Promise<FileOperationResult> {
  requireDesktop();
  return invoke<FileOperationResult>("copy_entries", { sources, destination });
}

export async function moveEntries(
  sources: string[],
  destination: string,
): Promise<FileOperationResult> {
  requireDesktop();
  return invoke<FileOperationResult>("move_entries", { sources, destination });
}

export async function trashEntries(
  paths: string[],
): Promise<FileOperationResult> {
  requireDesktop();
  return invoke<FileOperationResult>("trash_entries", { paths });
}

export async function deleteEntriesPermanently(
  paths: string[],
): Promise<FileOperationResult> {
  requireDesktop();
  return invoke<FileOperationResult>("delete_entries_permanently", {
    paths,
    confirmed: true,
  });
}

import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
  open: vi.fn(),
  save: vi.fn(),
  openUrl: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: mocks.invoke,
  isTauri: mocks.isTauri,
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: mocks.open,
  save: mocks.save,
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: mocks.openUrl,
}));

import {
  chooseAudioFile,
  chooseAudioOutput,
  chooseDirectory,
  chooseDocxOutput,
  chooseImageFiles,
  chooseOfficeFile,
  chooseDependencyExecutable,
  choosePdfFiles,
  choosePdfOutput,
  clearJobHistory,
  deleteJobHistory,
  enqueueAudioFolderConversion,
  enqueueImageFolderToPdf,
  enqueueOfficeFolderToPdf,
  enqueuePdfFolderToDocx,
  enqueuePdfFolderToImages,
  clearJobHistorySection,
  openExternalUrl,
  setDependencyOverride,
} from "./backend";

describe("native converter dialogs", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.isTauri.mockReturnValue(true);
  });

  it("opens image input selection and PDF output selection", async () => {
    mocks.open.mockResolvedValueOnce([
      "C:\\input\\one.png",
      "C:\\input\\two.jpg",
    ]);
    mocks.save.mockResolvedValueOnce("C:\\output\\images.pdf");

    await expect(chooseImageFiles()).resolves.toEqual([
      "C:\\input\\one.png",
      "C:\\input\\two.jpg",
    ]);
    await expect(choosePdfOutput("images.pdf")).resolves.toBe(
      "C:\\output\\images.pdf",
    );
    expect(mocks.open).toHaveBeenCalledWith(
      expect.objectContaining({ directory: false, multiple: true }),
    );
    expect(mocks.save).toHaveBeenCalledWith(
      expect.objectContaining({ defaultPath: "images.pdf" }),
    );
  });

  it("opens PDF input, output-file, and split-output-folder selection", async () => {
    mocks.open
      .mockResolvedValueOnce(["C:\\input\\one.pdf", "C:\\input\\two.pdf"])
      .mockResolvedValueOnce("C:\\output\\split");
    mocks.save.mockResolvedValueOnce("C:\\output\\organized.pdf");

    await expect(choosePdfFiles(true)).resolves.toEqual([
      "C:\\input\\one.pdf",
      "C:\\input\\two.pdf",
    ]);
    await expect(choosePdfOutput("organized.pdf")).resolves.toBe(
      "C:\\output\\organized.pdf",
    );
    await expect(chooseDirectory()).resolves.toBe("C:\\output\\split");
    expect(mocks.open).toHaveBeenLastCalledWith({
      directory: true,
      multiple: false,
    });
  });

  it("opens audio input and output selection", async () => {
    mocks.open.mockResolvedValueOnce("C:\\input\\track.wav");
    mocks.save.mockResolvedValueOnce("C:\\output\\track.mp3");

    await expect(chooseAudioFile()).resolves.toBe("C:\\input\\track.wav");
    await expect(chooseAudioOutput("track.mp3", "mp3")).resolves.toBe(
      "C:\\output\\track.mp3",
    );
    expect(mocks.save).toHaveBeenCalledWith(
      expect.objectContaining({
        defaultPath: "track.mp3",
        filters: [{ name: "MP3", extensions: ["mp3"] }],
      }),
    );
  });

  it("offers M4A as an AAC-container output", async () => {
    mocks.save.mockResolvedValueOnce("C:\\output\\track.m4a");

    await expect(chooseAudioOutput("track.m4a", "m4a")).resolves.toBe(
      "C:\\output\\track.m4a",
    );
    expect(mocks.save).toHaveBeenCalledWith(
      expect.objectContaining({
        defaultPath: "track.m4a",
        filters: [{ name: "M4A", extensions: ["m4a"] }],
      }),
    );
  });

  it("opens Office input and PDF output selection", async () => {
    mocks.open.mockResolvedValueOnce("C:\\input\\report.docx");
    mocks.save.mockResolvedValueOnce("C:\\output\\report.pdf");

    await expect(chooseOfficeFile()).resolves.toBe("C:\\input\\report.docx");
    await expect(choosePdfOutput("report.pdf")).resolves.toBe(
      "C:\\output\\report.pdf",
    );
  });

  it("opens dependency executables and official URLs through native plugins", async () => {
    mocks.open.mockResolvedValueOnce(
      "C:\\Apps\\LibreOffice\\program\\soffice.exe",
    );

    await expect(chooseDependencyExecutable("libreoffice")).resolves.toContain(
      "soffice.exe",
    );
    await openExternalUrl(
      "https://www.libreoffice.org/download/download-libreoffice/",
    );
    expect(mocks.open).toHaveBeenCalledWith(
      expect.objectContaining({
        filters: [{ name: "soffice.exe", extensions: ["exe"] }],
      }),
    );
    expect(mocks.openUrl).toHaveBeenCalledWith(
      "https://www.libreoffice.org/download/download-libreoffice/",
    );
  });

  it("persists and resets dependency overrides through the backend", async () => {
    mocks.invoke.mockResolvedValue({});
    await setDependencyOverride("python", "C:\\Python312\\python.exe");
    await setDependencyOverride("python", null);
    expect(mocks.invoke).toHaveBeenNthCalledWith(1, "set_dependency_override", {
      dependency: "python",
      path: "C:\\Python312\\python.exe",
    });
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, "set_dependency_override", {
      dependency: "python",
      path: null,
    });
  });

  it("opens PDF-to-Word input and DOCX output selection", async () => {
    mocks.open.mockResolvedValueOnce("C:\\input\\document.pdf");
    mocks.save.mockResolvedValueOnce("C:\\output\\document.docx");

    await expect(choosePdfFiles(false)).resolves.toEqual([
      "C:\\input\\document.pdf",
    ]);
    await expect(chooseDocxOutput("document.docx")).resolves.toBe(
      "C:\\output\\document.docx",
    );
    expect(mocks.save).toHaveBeenCalledWith(
      expect.objectContaining({
        defaultPath: "document.docx",
        filters: [{ name: "Word document", extensions: ["docx"] }],
      }),
    );
  });

  it("invokes persistent job history deletion commands", async () => {
    mocks.invoke
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(4)
      .mockResolvedValueOnce(2);

    await expect(deleteJobHistory("finished-job")).resolves.toBeUndefined();
    await expect(clearJobHistory()).resolves.toBe(4);
    await expect(clearJobHistorySection("failed")).resolves.toBe(2);
    expect(mocks.invoke).toHaveBeenNthCalledWith(1, "delete_job_history", {
      id: "finished-job",
    });
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, "clear_job_history");
    expect(mocks.invoke).toHaveBeenNthCalledWith(
      3,
      "clear_job_history_section",
      { section: "failed" },
    );
  });

  it("invokes every non-audio folder batch command", async () => {
    mocks.invoke.mockResolvedValue([]);

    await enqueueImageFolderToPdf("C:\\images", "C:\\output", "combine");
    await enqueuePdfFolderToImages("C:\\pdfs", "C:\\images", "jpeg", 300);
    await enqueueOfficeFolderToPdf("C:\\office", "C:\\output");
    await enqueuePdfFolderToDocx("C:\\pdfs", "C:\\word");

    expect(mocks.invoke).toHaveBeenNthCalledWith(
      1,
      "enqueue_image_folder_to_pdf",
      {
        inputDirectory: "C:\\images",
        outputDirectory: "C:\\output",
        mode: "combine",
      },
    );
    expect(mocks.invoke).toHaveBeenNthCalledWith(
      2,
      "enqueue_pdf_folder_to_images",
      {
        inputDirectory: "C:\\pdfs",
        outputDirectory: "C:\\images",
        format: "jpeg",
        dpi: 300,
      },
    );
    expect(mocks.invoke).toHaveBeenNthCalledWith(
      3,
      "enqueue_office_folder_to_pdf",
      { inputDirectory: "C:\\office", outputDirectory: "C:\\output" },
    );
    expect(mocks.invoke).toHaveBeenNthCalledWith(
      4,
      "enqueue_pdf_folder_to_docx",
      { inputDirectory: "C:\\pdfs", outputDirectory: "C:\\word" },
    );
  });

  it("enqueues every supported audio file in a folder through the backend", async () => {
    const jobs = [{ id: "folder-job-1" }, { id: "folder-job-2" }];
    mocks.invoke.mockResolvedValueOnce(jobs);

    await expect(
      enqueueAudioFolderConversion(
        "C:\\input\\album",
        "C:\\output\\album",
        "mp3",
      ),
    ).resolves.toEqual(jobs);
    expect(mocks.invoke).toHaveBeenCalledWith(
      "enqueue_audio_folder_conversion",
      {
        inputDirectory: "C:\\input\\album",
        outputDirectory: "C:\\output\\album",
        format: "mp3",
      },
    );
  });
});

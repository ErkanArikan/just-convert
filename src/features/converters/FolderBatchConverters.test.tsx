import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import i18n from "../../i18n";

const mocks = vi.hoisted(() => ({
  chooseDirectory: vi.fn(),
  chooseDocxOutput: vi.fn(),
  chooseImageFiles: vi.fn(),
  chooseOfficeFile: vi.fn(),
  choosePdfFiles: vi.fn(),
  choosePdfOutput: vi.fn(),
  enqueueImageFolderToPdf: vi.fn(),
  enqueueImagesToPdf: vi.fn(),
  enqueueOfficeFolderToPdf: vi.fn(),
  enqueueOfficeToPdf: vi.fn(),
  enqueuePdfFolderToDocx: vi.fn(),
  enqueuePdfFolderToImages: vi.fn(),
  enqueuePdfToDocx: vi.fn(),
  enqueuePdfToImages: vi.fn(),
  getPdfPageCount: vi.fn(),
}));

vi.mock("../../services/backend", () => mocks);

import { ImagesToPdf } from "./ImagesToPdf";
import { OfficeToPdf } from "./OfficeToPdf";
import { PdfToDocx } from "./PdfToDocx";
import { PdfToImages } from "./PdfToImages";

const commonProps = {
  desktopAvailable: true,
  onBack: () => undefined,
  onViewJobs: () => undefined,
};

describe("folder batch converter workflows", () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    await i18n.changeLanguage("en");
  });

  it("combines a folder of images into one PDF job", async () => {
    const user = userEvent.setup();
    mocks.chooseDirectory
      .mockResolvedValueOnce("C:\\Images")
      .mockResolvedValueOnce("C:\\Output");
    mocks.enqueueImageFolderToPdf.mockResolvedValueOnce([{ id: "images" }]);
    render(<ImagesToPdf {...commonProps} />);

    await user.click(screen.getByRole("tab", { name: "Folder batch" }));
    await user.click(
      screen.getByRole("button", { name: "Choose source folder" }),
    );
    await user.click(
      screen.getByRole("button", { name: "Choose output folder" }),
    );
    await user.click(screen.getByRole("button", { name: "Add to queue" }));

    expect(mocks.enqueueImageFolderToPdf).toHaveBeenCalledWith(
      "C:\\Images",
      "C:\\Output",
      "combine",
    );
  });

  it("queues one PDF job per image when separate output is selected", async () => {
    const user = userEvent.setup();
    mocks.chooseDirectory
      .mockResolvedValueOnce("C:\\Images")
      .mockResolvedValueOnce("C:\\Output");
    mocks.enqueueImageFolderToPdf.mockResolvedValueOnce([
      { id: "image-1" },
      { id: "image-2" },
    ]);
    render(<ImagesToPdf {...commonProps} />);

    await user.click(screen.getByRole("tab", { name: "Folder batch" }));
    expect(
      screen.getByRole("radio", {
        name: /Combine all images into one PDF/,
      }),
    ).toBeChecked();
    await user.click(screen.getByRole("radio", { name: /One PDF per image/ }));
    await user.click(
      screen.getByRole("button", { name: "Choose source folder" }),
    );
    await user.click(
      screen.getByRole("button", { name: "Choose output folder" }),
    );
    await user.click(screen.getByRole("button", { name: "Add to queue" }));

    expect(mocks.enqueueImageFolderToPdf).toHaveBeenCalledWith(
      "C:\\Images",
      "C:\\Output",
      "separate",
    );
    expect(
      screen.getByText("2 separate image-to-PDF jobs added."),
    ).toBeInTheDocument();
  });

  it("queues each PDF for image export with the selected quality", async () => {
    const user = userEvent.setup();
    mocks.chooseDirectory
      .mockResolvedValueOnce("C:\\PDFs")
      .mockResolvedValueOnce("C:\\Output");
    mocks.enqueuePdfFolderToImages.mockResolvedValueOnce([
      { id: "pdf-1" },
      { id: "pdf-2" },
    ]);
    render(<PdfToImages {...commonProps} />);

    await user.click(screen.getByRole("tab", { name: "Folder batch" }));
    await user.click(
      screen.getByRole("button", { name: "Choose source folder" }),
    );
    await user.selectOptions(screen.getByLabelText("Image format"), "jpeg");
    await user.selectOptions(screen.getByLabelText("Resolution"), "300");
    await user.click(screen.getByRole("button", { name: "Choose folder" }));
    await user.click(screen.getByRole("button", { name: "Add to queue" }));

    expect(mocks.enqueuePdfFolderToImages).toHaveBeenCalledWith(
      "C:\\PDFs",
      "C:\\Output",
      "jpeg",
      300,
    );
  });

  it("queues each supported Office document", async () => {
    const user = userEvent.setup();
    mocks.chooseDirectory
      .mockResolvedValueOnce("C:\\Office")
      .mockResolvedValueOnce("C:\\Output");
    mocks.enqueueOfficeFolderToPdf.mockResolvedValueOnce([{ id: "office" }]);
    render(<OfficeToPdf {...commonProps} engineAvailable />);

    await user.click(screen.getByRole("tab", { name: "Folder batch" }));
    await user.click(
      screen.getByRole("button", { name: "Choose source folder" }),
    );
    await user.click(
      screen.getByRole("button", { name: "Choose output folder" }),
    );
    await user.click(screen.getByRole("button", { name: "Add to queue" }));

    expect(mocks.enqueueOfficeFolderToPdf).toHaveBeenCalledWith(
      "C:\\Office",
      "C:\\Output",
    );
  });

  it("queues each PDF for Word conversion", async () => {
    const user = userEvent.setup();
    mocks.chooseDirectory
      .mockResolvedValueOnce("C:\\PDFs")
      .mockResolvedValueOnce("C:\\Word");
    mocks.enqueuePdfFolderToDocx.mockResolvedValueOnce([{ id: "word" }]);
    render(<PdfToDocx {...commonProps} engineAvailable />);

    await user.click(screen.getByRole("tab", { name: "Folder batch" }));
    await user.click(
      screen.getByRole("button", { name: "Choose source folder" }),
    );
    await user.click(
      screen.getByRole("button", { name: "Choose output folder" }),
    );
    await user.click(screen.getByRole("button", { name: "Add to queue" }));

    expect(mocks.enqueuePdfFolderToDocx).toHaveBeenCalledWith(
      "C:\\PDFs",
      "C:\\Word",
    );
  });
});

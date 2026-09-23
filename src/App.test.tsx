import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { OfficeToPdf } from "./features/converters/OfficeToPdf";
import { PdfToDocx } from "./features/converters/PdfToDocx";
import { Settings } from "./features/settings/Settings";
import i18n from "./i18n";
import { usePreferencesStore } from "./store/preferences";

describe("App foundation", () => {
  beforeEach(async () => {
    localStorage.clear();
    usePreferencesStore.setState({ language: "en", theme: "system" });
    await i18n.changeLanguage("en");
  });

  it("shows the local conversion capability registry", async () => {
    render(<App />);

    expect(
      await screen.findByRole("heading", { name: "Conversion tools" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Convert audio")).toBeInTheDocument();
    expect(screen.getByText("FFmpeg · Bundled")).toBeInTheDocument();
  });

  it("keeps the v1 navigation limited to functional workspaces", async () => {
    render(<App />);

    await screen.findByRole("heading", { name: "Conversion tools" });
    const navigation = screen.getByRole("navigation", {
      name: "Primary navigation",
    });
    expect(within(navigation).getAllByRole("button")).toHaveLength(4);
    expect(
      within(navigation).getByRole("button", { name: "Files" }),
    ).toBeInTheDocument();
    expect(
      within(navigation).getByRole("button", { name: "Convert" }),
    ).toBeInTheDocument();
    expect(
      within(navigation).getByRole("button", { name: "Jobs" }),
    ).toBeInTheDocument();
    expect(
      within(navigation).getByRole("button", { name: "Settings" }),
    ).toBeInTheDocument();
  });

  it("switches the interface to Turkish", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.selectOptions(screen.getByLabelText("Language"), "tr");

    expect(await screen.findByText("Dönüştürme araçları")).toBeInTheDocument();
    expect(screen.getByText("Ses dönüştür")).toBeInTheDocument();
  });

  it("keeps local folder selection disabled in a browser preview", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Files" }));

    expect(
      screen.getByRole("heading", { name: "Choose a folder to begin" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Choose folder" }),
    ).toBeDisabled();
  });

  it("opens the audio workflow while keeping native selection disabled in preview", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(
      await screen.findByRole("button", { name: /Convert audio/ }),
    );

    expect(
      screen.getByRole("heading", { name: "Convert audio" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Choose audio" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Add to queue" })).toBeDisabled();
  });

  it("opens the native images-to-PDF workflow", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(
      await screen.findByRole("button", { name: /Images to PDF/ }),
    );

    expect(
      screen.getByRole("heading", { name: "Images to PDF" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Automatic A4 layout")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Add images" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Add to queue" })).toBeDisabled();
  });

  it("opens all four PDF organization modes", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(
      await screen.findByRole("button", { name: /Organize PDF/ }),
    );

    expect(
      screen.getByRole("heading", { name: "Organize PDF" }),
    ).toBeInTheDocument();
    expect(screen.getAllByRole("tab")).toHaveLength(4);
    expect(screen.getByRole("button", { name: "Add PDFs" })).toBeDisabled();
    await user.click(screen.getByRole("tab", { name: "Split" }));
    expect(screen.getByText("Pages per file")).toBeInTheDocument();
    await user.click(screen.getByRole("tab", { name: "Reorder" }));
    expect(screen.getByText("New page order")).toBeInTheDocument();
    await user.click(screen.getByRole("tab", { name: "Rotate" }));
    expect(screen.getByText("Clockwise rotation")).toBeInTheDocument();
  });

  it("opens the local PDF-to-images workflow", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(
      await screen.findByRole("button", { name: /PDF to images/ }),
    );

    expect(
      screen.getByRole("heading", { name: "PDF to images" }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText("Image format")).toHaveValue("png");
    expect(screen.getByLabelText("Resolution")).toHaveValue("150");
    expect(screen.getByRole("button", { name: "Choose PDF" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Add to queue" })).toBeDisabled();
  });

  it("shows an actionable LibreOffice installation message when detection fails", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(
      await screen.findByRole("button", { name: /Office to PDF/ }),
    );

    expect(screen.getByText("LibreOffice is required")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Download LibreOffice" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Set path manually" }),
    ).toBeInTheDocument();
  });

  it("renders the ready Office workflow with desktop actions guarded", () => {
    render(
      <OfficeToPdf
        desktopAvailable={false}
        engineAvailable
        onBack={() => undefined}
        onViewJobs={() => undefined}
      />,
    );

    expect(
      screen.getByRole("button", { name: "Choose document" }),
    ).toBeDisabled();
    expect(screen.getByRole("button", { name: "Add to queue" })).toBeDisabled();
  });

  it("shows actionable Python package guidance for PDF-to-Word", async () => {
    const user = userEvent.setup();
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    render(<App />);

    await user.click(
      await screen.findByRole("button", { name: /PDF to Word/ }),
    );

    expect(
      screen.getByText("Python conversion dependencies are required"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("pip install pymupdf pdf2docx"),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Download Python" }),
    ).toBeInTheDocument();
    expect(
      screen
        .getByRole("button", { name: "Download Python" })
        .closest(".python-dependency-actions"),
    ).toContainElement(screen.getByText("pip install pymupdf pdf2docx"));
    expect(
      screen.getByRole("button", { name: "Set path manually" }),
    ).toBeInTheDocument();
    await user.click(
      screen.getByRole("button", { name: "Copy install command" }),
    );
    expect(writeText).toHaveBeenCalledWith("pip install pymupdf pdf2docx");
    expect(screen.getByText("Copied")).toBeInTheDocument();
  });

  it("lets automatic dependency detection be run again from Settings", async () => {
    const user = userEvent.setup();
    const onRescan = vi.fn().mockResolvedValue(undefined);
    const missing = (id: "libreoffice" | "python") => ({
      id,
      automaticPath: null,
      manualPath: null,
      manualPathValid: false,
      resolvedPath: null,
      ready: false,
      issue: null,
    });
    render(
      <Settings
        dependencies={{
          libreoffice: missing("libreoffice"),
          python: missing("python"),
        }}
        desktopAvailable
        onBrowse={vi.fn()}
        onRescan={onRescan}
        onReset={vi.fn()}
      />,
    );

    await user.click(
      screen.getAllByRole("button", { name: "Search again" })[0],
    );
    expect(onRescan).toHaveBeenCalledOnce();
  });

  it("warns without discarding a manually selected Python that fails the version probe", () => {
    const missing = {
      id: "libreoffice" as const,
      automaticPath: null,
      manualPath: null,
      manualPathValid: false,
      resolvedPath: null,
      ready: false,
      issue: null,
    };
    render(
      <Settings
        dependencies={{
          libreoffice: missing,
          python: {
            id: "python",
            automaticPath: null,
            manualPath: "C:\\CustomPython\\python.exe",
            manualPathValid: true,
            resolvedPath: "C:\\CustomPython\\python.exe",
            ready: false,
            issue: "version_check_failed",
          },
        }}
        desktopAvailable
        onBrowse={vi.fn()}
        onRescan={vi.fn()}
        onReset={vi.fn()}
      />,
    );

    expect(
      screen.getByText(
        "Python 3.10+ could not be confirmed. The manual override was saved and will still be used.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText("C:\\CustomPython\\python.exe"),
    ).toBeInTheDocument();
  });

  it("renders PDF-to-Word quality expectations when dependencies are ready", () => {
    render(
      <PdfToDocx
        desktopAvailable={false}
        engineAvailable
        onBack={() => undefined}
        onViewJobs={() => undefined}
      />,
    );

    expect(
      screen.getByText("Best-effort layout conversion"),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Choose PDF" })).toBeDisabled();
  });

  it("shows an empty local job queue in browser preview", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Jobs" }));

    expect(
      await screen.findByRole("heading", { name: "No conversion jobs yet" }),
    ).toBeInTheDocument();
  });
});

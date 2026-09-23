import { useState } from "react";
import { ArrowLeft, Files, FolderOpen, ListChecks } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  chooseDirectory,
  choosePdfFiles,
  enqueuePdfFolderToImages,
  enqueuePdfToImages,
  getPdfPageCount,
} from "../../services/backend";
import { SourceModeTabs, type SourceMode } from "./SourceModeTabs";

function fileName(path: string) {
  return path.split(/[\\/]/).pop() ?? path;
}

interface PdfToImagesProps {
  desktopAvailable: boolean;
  onBack: () => void;
  onViewJobs: () => void;
}

export function PdfToImages({
  desktopAvailable,
  onBack,
  onViewJobs,
}: PdfToImagesProps) {
  const { t } = useTranslation();
  const [mode, setMode] = useState<SourceMode>("file");
  const [inputPath, setInputPath] = useState("");
  const [pageCount, setPageCount] = useState<number | null>(null);
  const [outputDirectory, setOutputDirectory] = useState("");
  const [format, setFormat] = useState<"png" | "jpeg">("png");
  const [dpi, setDpi] = useState<150 | 300>(150);
  const [queuedCount, setQueuedCount] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function selectInput() {
    setBusy(true);
    setError(null);
    try {
      const selected =
        mode === "file"
          ? (await choosePdfFiles(false))[0]
          : await chooseDirectory();
      if (!selected) return;
      setInputPath(selected);
      setPageCount(mode === "file" ? await getPdfPageCount(selected) : null);
      setOutputDirectory("");
      setQueuedCount(0);
    } catch (reason) {
      setError(String(reason));
      if (mode === "file") {
        setInputPath("");
        setPageCount(null);
      }
    } finally {
      setBusy(false);
    }
  }

  async function selectOutputDirectory() {
    setError(null);
    try {
      const selected = await chooseDirectory();
      if (selected) setOutputDirectory(selected);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function enqueue() {
    if (!inputPath || !outputDirectory) return;
    setBusy(true);
    setError(null);
    try {
      if (mode === "file") {
        await enqueuePdfToImages(inputPath, outputDirectory, format, dpi);
        setQueuedCount(1);
      } else {
        const jobs = await enqueuePdfFolderToImages(
          inputPath,
          outputDirectory,
          format,
          dpi,
        );
        setQueuedCount(jobs.length);
      }
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }

  function selectMode(nextMode: SourceMode) {
    if (nextMode === mode) return;
    setMode(nextMode);
    setInputPath("");
    setPageCount(null);
    setOutputDirectory("");
    setQueuedCount(0);
    setError(null);
  }

  return (
    <div className="tool-workspace">
      <button className="back-button" onClick={onBack} type="button">
        <ArrowLeft size={16} />
        {t("pdfImages.back")}
      </button>
      <div className="tool-heading">
        <span className="converter-icon">
          <Files size={20} />
        </span>
        <div>
          <p className="eyebrow accent">PDFium</p>
          <h2>{t("pdfImages.title")}</h2>
          <p>{t("pdfImages.description")}</p>
        </div>
      </div>

      <SourceModeTabs busy={busy} mode={mode} onChange={selectMode} />

      <div className="audio-form">
        <div className="audio-step">
          <span className="step-number">1</span>
          <div>
            <strong>
              {t(mode === "file" ? "pdfImages.input" : "pdfImages.folderInput")}
            </strong>
            <p>
              {mode === "folder"
                ? t("pdfImages.folderInputHelp")
                : inputPath
                  ? t("pdfImages.pageCount", { count: pageCount ?? "…" })
                  : t("pdfImages.inputHelp")}
            </p>
          </div>
          <button
            disabled={!desktopAvailable || busy}
            onClick={() => void selectInput()}
            type="button"
          >
            <FolderOpen size={16} />
            {inputPath
              ? fileName(inputPath)
              : t(
                  mode === "file"
                    ? "pdfImages.choosePdf"
                    : "pdfImages.chooseSourceFolder",
                )}
          </button>
        </div>

        <div className="audio-step">
          <span className="step-number">2</span>
          <div>
            <strong>{t("pdfImages.format")}</strong>
            <p>{t("pdfImages.formatHelp")}</p>
          </div>
          <select
            aria-label={t("pdfImages.format")}
            disabled={busy}
            onChange={(event) =>
              setFormat(event.target.value as "png" | "jpeg")
            }
            value={format}
          >
            <option value="png">PNG</option>
            <option value="jpeg">JPEG</option>
          </select>
        </div>

        <div className="audio-step">
          <span className="step-number">3</span>
          <div>
            <strong>{t("pdfImages.resolution")}</strong>
            <p>{t("pdfImages.resolutionHelp")}</p>
          </div>
          <select
            aria-label={t("pdfImages.resolution")}
            disabled={busy}
            onChange={(event) =>
              setDpi(Number(event.target.value) as 150 | 300)
            }
            value={dpi}
          >
            <option value={150}>{t("pdfImages.standard")}</option>
            <option value={300}>{t("pdfImages.high")}</option>
          </select>
        </div>

        <div className="audio-step">
          <span className="step-number">4</span>
          <div>
            <strong>{t("pdfImages.output")}</strong>
            <p>
              {outputDirectory
                ? fileName(outputDirectory)
                : t(
                    mode === "file"
                      ? "pdfImages.outputHelp"
                      : "pdfImages.folderOutputHelp",
                  )}
            </p>
          </div>
          <button
            disabled={!inputPath || busy}
            onClick={() => void selectOutputDirectory()}
            type="button"
          >
            {t("pdfImages.chooseFolder")}
          </button>
        </div>
      </div>

      {error && (
        <div className="inline-error" role="alert">
          {error}
        </div>
      )}
      {queuedCount > 0 && (
        <div className="success-notice" role="status">
          <span>
            {mode === "file"
              ? t("pdfImages.queued")
              : t("pdfImages.batchQueued", { count: queuedCount })}
          </span>
          <button onClick={onViewJobs} type="button">
            <ListChecks size={15} />
            {t("pdfImages.viewJobs")}
          </button>
        </div>
      )}
      <div className="tool-actions">
        <span>{t("pdfImages.localNotice")}</span>
        <button
          className="primary-button"
          disabled={!inputPath || !outputDirectory || busy}
          onClick={() => void enqueue()}
          type="button"
        >
          {busy ? t("pdfImages.adding") : t("pdfImages.addToQueue")}
        </button>
      </div>
    </div>
  );
}

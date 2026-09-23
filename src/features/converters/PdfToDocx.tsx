import { useState } from "react";
import {
  ArrowLeft,
  Check,
  Copy,
  ExternalLink,
  FileText,
  FolderOpen,
  ListChecks,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  chooseDirectory,
  chooseDocxOutput,
  choosePdfFiles,
  enqueuePdfFolderToDocx,
  enqueuePdfToDocx,
  openExternalUrl,
} from "../../services/backend";
import { SourceModeTabs, type SourceMode } from "./SourceModeTabs";

const PYTHON_DOWNLOAD = "https://www.python.org/downloads/";
const INSTALL_COMMAND = "pip install pymupdf pdf2docx";

function fileName(path: string) {
  return path.split(/[\\/]/).pop() ?? path;
}

function outputSuggestion(input: string) {
  const withoutExtension = input.replace(/\.[^./\\]+$/, "");
  return `${withoutExtension || input}.docx`;
}

interface PdfToDocxProps {
  desktopAvailable: boolean;
  engineAvailable: boolean;
  onBack: () => void;
  onViewJobs: () => void;
  onSetManualPath?: () => void;
}

export function PdfToDocx({
  desktopAvailable,
  engineAvailable,
  onBack,
  onViewJobs,
  onSetManualPath,
}: PdfToDocxProps) {
  const { t } = useTranslation();
  const [mode, setMode] = useState<SourceMode>("file");
  const [inputPath, setInputPath] = useState("");
  const [outputPath, setOutputPath] = useState("");
  const [queuedCount, setQueuedCount] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copyStatus, setCopyStatus] = useState<"idle" | "copied" | "failed">(
    "idle",
  );

  async function copyInstallCommand() {
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(INSTALL_COMMAND);
      } else {
        const textarea = document.createElement("textarea");
        textarea.value = INSTALL_COMMAND;
        textarea.style.position = "fixed";
        textarea.style.opacity = "0";
        document.body.appendChild(textarea);
        textarea.select();
        const copied = document.execCommand("copy");
        textarea.remove();
        if (!copied) throw new Error("Copy command was rejected");
      }
      setCopyStatus("copied");
      window.setTimeout(() => setCopyStatus("idle"), 2000);
    } catch {
      setCopyStatus("failed");
    }
  }

  async function selectInput() {
    setError(null);
    try {
      const selected =
        mode === "file"
          ? (await choosePdfFiles(false))[0]
          : await chooseDirectory();
      if (!selected) return;
      setInputPath(selected);
      setOutputPath("");
      setQueuedCount(0);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function selectOutput() {
    if (!inputPath) return;
    setError(null);
    try {
      const selected =
        mode === "file"
          ? await chooseDocxOutput(outputSuggestion(inputPath))
          : await chooseDirectory();
      if (selected) setOutputPath(selected);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function enqueue() {
    if (!inputPath || !outputPath) return;
    setBusy(true);
    setError(null);
    try {
      if (mode === "file") {
        await enqueuePdfToDocx(inputPath, outputPath);
        setQueuedCount(1);
      } else {
        const jobs = await enqueuePdfFolderToDocx(inputPath, outputPath);
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
    setOutputPath("");
    setQueuedCount(0);
    setError(null);
  }

  return (
    <div className="tool-workspace">
      <button className="back-button" onClick={onBack} type="button">
        <ArrowLeft size={16} />
        {t("pdfDocx.back")}
      </button>
      <div className="tool-heading">
        <span className="converter-icon">
          <FileText size={20} />
        </span>
        <div>
          <p className="eyebrow accent">Python · PyMuPDF · pdf2docx</p>
          <h2>{t("pdfDocx.title")}</h2>
          <p>{t("pdfDocx.description")}</p>
        </div>
      </div>

      {!engineAvailable ? (
        <div
          className="dependency-notice dependency-notice-stacked"
          role="status"
        >
          <div>
            <strong>{t("pdfDocx.missing.title")}</strong>
            <p>{t("pdfDocx.missing.description")}</p>
          </div>
          <div className="dependency-actions python-dependency-actions">
            <button
              onClick={() => void openExternalUrl(PYTHON_DOWNLOAD)}
              type="button"
            >
              <ExternalLink size={15} />
              {t("pdfDocx.missing.python")}
            </button>
            {onSetManualPath && (
              <button onClick={onSetManualPath} type="button">
                <FolderOpen size={15} />
                {t("pdfDocx.missing.setPath")}
              </button>
            )}
            <div className="install-command">
              <code>{INSTALL_COMMAND}</code>
              <button
                aria-label={t("pdfDocx.missing.copyCommand")}
                onClick={() => void copyInstallCommand()}
                type="button"
              >
                {copyStatus === "copied" ? (
                  <Check size={15} />
                ) : (
                  <Copy size={15} />
                )}
                {t(
                  copyStatus === "copied"
                    ? "pdfDocx.missing.copied"
                    : copyStatus === "failed"
                      ? "pdfDocx.missing.copyFailed"
                      : "pdfDocx.missing.copy",
                )}
              </button>
            </div>
          </div>
        </div>
      ) : (
        <>
          <SourceModeTabs busy={busy} mode={mode} onChange={selectMode} />
          <div className="notice" role="note">
            <strong>{t("pdfDocx.quality.title")}</strong>
            <p>{t("pdfDocx.quality.description")}</p>
          </div>
          <div className="audio-form">
            <div className="audio-step">
              <span className="step-number">1</span>
              <div>
                <strong>
                  {t(mode === "file" ? "pdfDocx.input" : "pdfDocx.folderInput")}
                </strong>
                <p>
                  {t(
                    mode === "file"
                      ? "pdfDocx.inputHelp"
                      : "pdfDocx.folderInputHelp",
                  )}
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
                        ? "pdfDocx.chooseFile"
                        : "pdfDocx.chooseSourceFolder",
                    )}
              </button>
            </div>
            <div className="audio-step">
              <span className="step-number">2</span>
              <div>
                <strong>{t("pdfDocx.output")}</strong>
                <p>
                  {outputPath
                    ? fileName(outputPath)
                    : t(
                        mode === "file"
                          ? "pdfDocx.outputHelp"
                          : "pdfDocx.folderOutputHelp",
                      )}
                </p>
              </div>
              <button
                disabled={!inputPath || busy}
                onClick={() => void selectOutput()}
                type="button"
              >
                {t(
                  mode === "file"
                    ? "pdfDocx.chooseOutput"
                    : "pdfDocx.chooseOutputFolder",
                )}
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
                  ? t("pdfDocx.queued")
                  : t("pdfDocx.batchQueued", { count: queuedCount })}
              </span>
              <button onClick={onViewJobs} type="button">
                <ListChecks size={15} />
                {t("pdfDocx.viewJobs")}
              </button>
            </div>
          )}
          <div className="tool-actions">
            <span>{t("pdfDocx.localNotice")}</span>
            <button
              className="primary-button"
              disabled={!inputPath || !outputPath || busy}
              onClick={() => void enqueue()}
              type="button"
            >
              {busy ? t("pdfDocx.adding") : t("pdfDocx.addToQueue")}
            </button>
          </div>
        </>
      )}
    </div>
  );
}

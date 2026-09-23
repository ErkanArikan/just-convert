import { useState } from "react";
import {
  ArrowLeft,
  ExternalLink,
  FileOutput,
  FolderOpen,
  ListChecks,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  chooseDirectory,
  chooseOfficeFile,
  choosePdfOutput,
  enqueueOfficeFolderToPdf,
  enqueueOfficeToPdf,
  openExternalUrl,
} from "../../services/backend";
import { SourceModeTabs, type SourceMode } from "./SourceModeTabs";

const LIBREOFFICE_DOWNLOAD =
  "https://www.libreoffice.org/download/download-libreoffice/";

function fileName(path: string) {
  return path.split(/[\\/]/).pop() ?? path;
}

function outputSuggestion(input: string) {
  const withoutExtension = input.replace(/\.[^./\\]+$/, "");
  return `${withoutExtension || input}.pdf`;
}

interface OfficeToPdfProps {
  desktopAvailable: boolean;
  engineAvailable: boolean;
  onBack: () => void;
  onViewJobs: () => void;
  onSetManualPath?: () => void;
}

export function OfficeToPdf({
  desktopAvailable,
  engineAvailable,
  onBack,
  onViewJobs,
  onSetManualPath,
}: OfficeToPdfProps) {
  const { t } = useTranslation();
  const [mode, setMode] = useState<SourceMode>("file");
  const [inputPath, setInputPath] = useState("");
  const [outputPath, setOutputPath] = useState("");
  const [queuedCount, setQueuedCount] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function selectInput() {
    setError(null);
    try {
      const selected =
        mode === "file" ? await chooseOfficeFile() : await chooseDirectory();
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
          ? await choosePdfOutput(outputSuggestion(inputPath))
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
        await enqueueOfficeToPdf(inputPath, outputPath);
        setQueuedCount(1);
      } else {
        const jobs = await enqueueOfficeFolderToPdf(inputPath, outputPath);
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
        {t("office.back")}
      </button>
      <div className="tool-heading">
        <span className="converter-icon">
          <FileOutput size={20} />
        </span>
        <div>
          <p className="eyebrow accent">LibreOffice</p>
          <h2>{t("office.title")}</h2>
          <p>{t("office.description")}</p>
        </div>
      </div>

      {!engineAvailable ? (
        <div className="dependency-notice" role="status">
          <div>
            <strong>{t("office.missing.title")}</strong>
            <p>{t("office.missing.description")}</p>
          </div>
          <div className="dependency-actions">
            <button
              onClick={() => void openExternalUrl(LIBREOFFICE_DOWNLOAD)}
              type="button"
            >
              <ExternalLink size={15} />
              {t("office.missing.download")}
            </button>
            {onSetManualPath && (
              <button onClick={onSetManualPath} type="button">
                <FolderOpen size={15} />
                {t("office.missing.setPath")}
              </button>
            )}
          </div>
        </div>
      ) : (
        <>
          <SourceModeTabs busy={busy} mode={mode} onChange={selectMode} />
          <div className="audio-form">
            <div className="audio-step">
              <span className="step-number">1</span>
              <div>
                <strong>
                  {t(mode === "file" ? "office.input" : "office.folderInput")}
                </strong>
                <p>
                  {t(
                    mode === "file"
                      ? "office.inputHelp"
                      : "office.folderInputHelp",
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
                        ? "office.chooseFile"
                        : "office.chooseSourceFolder",
                    )}
              </button>
            </div>
            <div className="audio-step">
              <span className="step-number">2</span>
              <div>
                <strong>{t("office.output")}</strong>
                <p>
                  {outputPath
                    ? fileName(outputPath)
                    : t(
                        mode === "file"
                          ? "office.outputHelp"
                          : "office.folderOutputHelp",
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
                    ? "office.chooseOutput"
                    : "office.chooseOutputFolder",
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
                  ? t("office.queued")
                  : t("office.batchQueued", { count: queuedCount })}
              </span>
              <button onClick={onViewJobs} type="button">
                <ListChecks size={15} />
                {t("office.viewJobs")}
              </button>
            </div>
          )}
          <div className="tool-actions">
            <span>{t("office.localNotice")}</span>
            <button
              className="primary-button"
              disabled={!inputPath || !outputPath || busy}
              onClick={() => void enqueue()}
              type="button"
            >
              {busy ? t("office.adding") : t("office.addToQueue")}
            </button>
          </div>
        </>
      )}
    </div>
  );
}

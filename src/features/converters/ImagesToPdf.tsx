import { useState } from "react";
import {
  ArrowDown,
  ArrowLeft,
  ArrowUp,
  FileImage,
  FolderOpen,
  ListChecks,
  Trash2,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  chooseImageFiles,
  chooseDirectory,
  choosePdfOutput,
  enqueueImageFolderToPdf,
  enqueueImagesToPdf,
} from "../../services/backend";
import { SourceModeTabs, type SourceMode } from "./SourceModeTabs";

function fileName(path: string) {
  return path.split(/[\\/]/).pop() ?? path;
}

function outputSuggestion(input: string) {
  const withoutExtension = input.replace(/\.[^./\\]+$/, "");
  return `${withoutExtension || input}-images.pdf`;
}

interface ImagesToPdfProps {
  desktopAvailable: boolean;
  onBack: () => void;
  onViewJobs: () => void;
}

export function ImagesToPdf({
  desktopAvailable,
  onBack,
  onViewJobs,
}: ImagesToPdfProps) {
  const { t } = useTranslation();
  const [mode, setMode] = useState<SourceMode>("file");
  const [inputs, setInputs] = useState<string[]>([]);
  const [inputDirectory, setInputDirectory] = useState("");
  const [folderOutputMode, setFolderOutputMode] = useState<
    "combine" | "separate"
  >("combine");
  const [outputPath, setOutputPath] = useState("");
  const [queuedCount, setQueuedCount] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function selectInputs() {
    setError(null);
    try {
      if (mode === "folder") {
        const selected = await chooseDirectory();
        if (!selected) return;
        setInputDirectory(selected);
      } else {
        const selected = await chooseImageFiles();
        setInputs((current) => [...new Set([...current, ...selected])]);
      }
      setOutputPath("");
      setQueuedCount(0);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function selectOutput() {
    if (mode === "file" ? !inputs.length : !inputDirectory) return;
    setError(null);
    try {
      const selected =
        mode === "file"
          ? await choosePdfOutput(outputSuggestion(inputs[0]))
          : await chooseDirectory();
      if (selected) setOutputPath(selected);
    } catch (reason) {
      setError(String(reason));
    }
  }

  function moveInput(index: number, offset: number) {
    setInputs((current) => {
      const target = index + offset;
      if (target < 0 || target >= current.length) return current;
      const next = [...current];
      [next[index], next[target]] = [next[target], next[index]];
      return next;
    });
    setQueuedCount(0);
  }

  async function enqueue() {
    if ((mode === "file" ? !inputs.length : !inputDirectory) || !outputPath)
      return;
    setBusy(true);
    setError(null);
    try {
      if (mode === "file") {
        await enqueueImagesToPdf(inputs, outputPath);
        setQueuedCount(1);
      } else {
        const jobs = await enqueueImageFolderToPdf(
          inputDirectory,
          outputPath,
          folderOutputMode,
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
    setInputs([]);
    setInputDirectory("");
    setOutputPath("");
    setQueuedCount(0);
    setError(null);
  }

  return (
    <div className="tool-workspace">
      <button className="back-button" onClick={onBack} type="button">
        <ArrowLeft size={16} />
        {t("images.back")}
      </button>
      <div className="tool-heading">
        <span className="converter-icon">
          <FileImage size={20} />
        </span>
        <div>
          <p className="eyebrow accent">{t("images.engine")}</p>
          <h2>{t("images.title")}</h2>
          <p>{t("images.description")}</p>
        </div>
      </div>

      <SourceModeTabs busy={busy} mode={mode} onChange={selectMode} />

      {mode === "file" ? (
        <div className="pdf-form">
          <div className="pdf-form-heading">
            <div>
              <strong>{t("images.order")}</strong>
              <p>{t("images.orderHelp")}</p>
            </div>
            <button
              disabled={!desktopAvailable || busy}
              onClick={() => void selectInputs()}
              type="button"
            >
              <FolderOpen size={15} />
              {t("images.add")}
            </button>
          </div>
          <div className="pdf-input-list">
            {inputs.length ? (
              inputs.map((path, index) => (
                <div className="pdf-input-row" key={path}>
                  <span className="pdf-order">{index + 1}</span>
                  <span title={path}>{fileName(path)}</span>
                  <button
                    aria-label={t("images.moveUp", { name: fileName(path) })}
                    disabled={index === 0}
                    onClick={() => moveInput(index, -1)}
                    type="button"
                  >
                    <ArrowUp size={14} />
                  </button>
                  <button
                    aria-label={t("images.moveDown", { name: fileName(path) })}
                    disabled={index === inputs.length - 1}
                    onClick={() => moveInput(index, 1)}
                    type="button"
                  >
                    <ArrowDown size={14} />
                  </button>
                  <button
                    aria-label={t("images.remove", { name: fileName(path) })}
                    onClick={() => {
                      setInputs((current) =>
                        current.filter((item) => item !== path),
                      );
                      setOutputPath("");
                      setQueuedCount(0);
                    }}
                    type="button"
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              ))
            ) : (
              <span className="pdf-list-empty">{t("images.empty")}</span>
            )}
          </div>
          <div className="image-page-note">
            <strong>{t("images.pageLayout")}</strong>
            <p>{t("images.pageLayoutHelp")}</p>
          </div>
          <div className="pdf-output-row">
            <div>
              <strong>{t("images.output")}</strong>
              <p>
                {outputPath ? fileName(outputPath) : t("images.outputHelp")}
              </p>
            </div>
            <button
              disabled={!inputs.length || busy}
              onClick={() => void selectOutput()}
              type="button"
            >
              {t("images.chooseOutput")}
            </button>
          </div>
        </div>
      ) : (
        <div className="audio-form">
          <div className="audio-step">
            <span className="step-number">1</span>
            <div>
              <strong>{t("images.folderInput")}</strong>
              <p>{t("images.folderInputHelp")}</p>
            </div>
            <button
              disabled={!desktopAvailable || busy}
              onClick={() => void selectInputs()}
              type="button"
            >
              <FolderOpen size={15} />
              {inputDirectory
                ? t("images.changeFolder")
                : t("images.chooseSourceFolder")}
            </button>
            {inputDirectory && (
              <span className="selected-file" title={inputDirectory}>
                {fileName(inputDirectory)}
              </span>
            )}
          </div>
          <div className="audio-step">
            <span className="step-number">2</span>
            <div>
              <strong>{t("images.folderMode")}</strong>
              <p>{t("images.folderModeHelp")}</p>
            </div>
            <div
              aria-label={t("images.folderMode")}
              className="image-batch-options"
              role="radiogroup"
            >
              <label data-active={folderOutputMode === "combine"}>
                <input
                  checked={folderOutputMode === "combine"}
                  disabled={busy}
                  name="image-folder-output-mode"
                  onChange={() => {
                    setFolderOutputMode("combine");
                    setQueuedCount(0);
                  }}
                  type="radio"
                  value="combine"
                />
                <span>
                  <strong>{t("images.combine")}</strong>
                  <small>{t("images.combineHelp")}</small>
                </span>
              </label>
              <label data-active={folderOutputMode === "separate"}>
                <input
                  checked={folderOutputMode === "separate"}
                  disabled={busy}
                  name="image-folder-output-mode"
                  onChange={() => {
                    setFolderOutputMode("separate");
                    setQueuedCount(0);
                  }}
                  type="radio"
                  value="separate"
                />
                <span>
                  <strong>{t("images.separate")}</strong>
                  <small>{t("images.separateHelp")}</small>
                </span>
              </label>
            </div>
          </div>
          <div className="audio-step">
            <span className="step-number">3</span>
            <div>
              <strong>{t("images.folderOutput")}</strong>
              <p>
                {t(
                  folderOutputMode === "combine"
                    ? "images.folderOutputHelpCombine"
                    : "images.folderOutputHelpSeparate",
                )}
              </p>
            </div>
            <button
              disabled={!inputDirectory || busy}
              onClick={() => void selectOutput()}
              type="button"
            >
              {t("images.chooseOutputFolder")}
            </button>
            {outputPath && (
              <span className="selected-file" title={outputPath}>
                {fileName(outputPath)}
              </span>
            )}
          </div>
        </div>
      )}

      {error && (
        <div className="inline-error" role="alert">
          {error}
        </div>
      )}
      {queuedCount > 0 && (
        <div className="success-notice" role="status">
          <span>
            {mode === "file"
              ? t("images.queued")
              : t(
                  folderOutputMode === "combine"
                    ? "images.batchQueuedCombine"
                    : "images.batchQueuedSeparate",
                  { count: queuedCount },
                )}
          </span>
          <button onClick={onViewJobs} type="button">
            <ListChecks size={15} />
            {t("images.viewJobs")}
          </button>
        </div>
      )}
      <div className="tool-actions">
        <span>{t("images.localNotice")}</span>
        <button
          className="primary-button"
          disabled={
            (mode === "file" ? !inputs.length : !inputDirectory) ||
            !outputPath ||
            busy
          }
          onClick={() => void enqueue()}
          type="button"
        >
          {busy ? t("images.adding") : t("images.addToQueue")}
        </button>
      </div>
    </div>
  );
}

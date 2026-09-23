import { useState } from "react";
import { ArrowLeft, FileAudio, FolderOpen, ListChecks } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  chooseAudioFile,
  chooseAudioOutput,
  chooseDirectory,
  enqueueAudioConversion,
  enqueueAudioFolderConversion,
} from "../../services/backend";
import type { AudioFormat } from "../../types/jobs";
import { SourceModeTabs, type SourceMode } from "./SourceModeTabs";

const FORMATS: AudioFormat[] = ["mp3", "wav", "flac", "aac", "m4a", "ogg"];

function outputSuggestion(input: string, format: AudioFormat) {
  const withoutExtension = input.replace(/\.[^./\\]+$/, "");
  return `${withoutExtension || input}.${format}`;
}

function fileName(path: string) {
  return path.split(/[\\/]/).pop() ?? path;
}

interface AudioConverterProps {
  desktopAvailable: boolean;
  onBack: () => void;
  onViewJobs: () => void;
}

export function AudioConverter({
  desktopAvailable,
  onBack,
  onViewJobs,
}: AudioConverterProps) {
  const { t } = useTranslation();
  const [mode, setMode] = useState<SourceMode>("file");
  const [inputPath, setInputPath] = useState("");
  const [outputPath, setOutputPath] = useState("");
  const [format, setFormat] = useState<AudioFormat>("mp3");
  const [queuedCount, setQueuedCount] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function selectInput() {
    setError(null);
    try {
      const selected =
        mode === "file" ? await chooseAudioFile() : await chooseDirectory();
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
          ? await chooseAudioOutput(outputSuggestion(inputPath, format), format)
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
        await enqueueAudioConversion(inputPath, outputPath, format);
        setQueuedCount(1);
      } else {
        const jobs = await enqueueAudioFolderConversion(
          inputPath,
          outputPath,
          format,
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
    setOutputPath("");
    setQueuedCount(0);
    setError(null);
  }

  return (
    <div className="tool-workspace">
      <button className="back-button" onClick={onBack} type="button">
        <ArrowLeft size={16} />
        {t("audio.back")}
      </button>
      <div className="tool-heading">
        <span className="converter-icon">
          <FileAudio size={20} />
        </span>
        <div>
          <p className="eyebrow accent">FFmpeg</p>
          <h2>{t("audio.title")}</h2>
          <p>{t("audio.description")}</p>
        </div>
      </div>

      <SourceModeTabs busy={busy} mode={mode} onChange={selectMode} />

      <div className="audio-form">
        <div className="audio-step">
          <span className="step-number">1</span>
          <div>
            <strong>
              {t(
                mode === "file"
                  ? "audio.input.label"
                  : "audio.folderInput.label",
              )}
            </strong>
            <p>
              {t(
                mode === "file" ? "audio.input.help" : "audio.folderInput.help",
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
              ? t("audio.input.change")
              : t(
                  mode === "file"
                    ? "audio.input.choose"
                    : "audio.folderInput.choose",
                )}
          </button>
          {inputPath && (
            <span className="selected-file" title={inputPath}>
              {fileName(inputPath)}
            </span>
          )}
        </div>

        <div className="audio-step">
          <span className="step-number">2</span>
          <div>
            <strong>{t("audio.format.label")}</strong>
            <p>{t("audio.format.help")}</p>
          </div>
          <select
            aria-label={t("audio.format.label")}
            disabled={busy}
            onChange={(event) => {
              setFormat(event.target.value as AudioFormat);
              if (mode === "file") setOutputPath("");
              setQueuedCount(0);
            }}
            value={format}
          >
            {FORMATS.map((item) => (
              <option key={item} value={item}>
                {item.toUpperCase()}
              </option>
            ))}
          </select>
        </div>

        <div className="audio-step">
          <span className="step-number">3</span>
          <div>
            <strong>
              {t(
                mode === "file"
                  ? "audio.output.label"
                  : "audio.folderOutput.label",
              )}
            </strong>
            <p>
              {t(
                mode === "file"
                  ? "audio.output.help"
                  : "audio.folderOutput.help",
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
                ? "audio.output.choose"
                : "audio.folderOutput.choose",
            )}
          </button>
          {outputPath && (
            <span className="selected-file" title={outputPath}>
              {fileName(outputPath)}
            </span>
          )}
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
              ? t("audio.queued")
              : t("audio.batchQueued", { count: queuedCount })}
          </span>
          <button onClick={onViewJobs} type="button">
            <ListChecks size={15} />
            {t("audio.viewJobs")}
          </button>
        </div>
      )}
      <div className="tool-actions">
        <span>{t("audio.localNotice")}</span>
        <button
          className="primary-button"
          disabled={!inputPath || !outputPath || busy}
          onClick={() => void enqueue()}
          type="button"
        >
          {busy ? t("audio.adding") : t("audio.convert")}
        </button>
      </div>
    </div>
  );
}

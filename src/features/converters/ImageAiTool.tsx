import { useState } from "react";
import {
  ArrowLeft,
  Eraser,
  FolderOpen,
  ListChecks,
  Maximize2,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  chooseImageFile,
  chooseImageOutput,
  enqueueImageAi,
} from "../../services/backend";
import type { ImageAiOperation } from "../../types/jobs";

type Mode = "remove-background" | "upscale-image";
type Scale = 2 | 4;
type Format = "png" | "jpg";

function outputSuggestion(
  input: string,
  mode: Mode,
  scale: Scale,
  format: Format,
) {
  const withoutExtension = input.replace(/\.[^./\\]+$/, "") || input;
  return mode === "remove-background"
    ? `${withoutExtension}-cutout.png`
    : `${withoutExtension}-${scale}x.${format}`;
}

function fileName(path: string) {
  return path.split(/[\\/]/).pop() ?? path;
}

interface ImageAiToolProps {
  desktopAvailable: boolean;
  mode: Mode;
  onBack: () => void;
  onViewJobs: () => void;
}

export function ImageAiTool({
  desktopAvailable,
  mode,
  onBack,
  onViewJobs,
}: ImageAiToolProps) {
  const { t } = useTranslation();
  const key = mode === "remove-background" ? "removeBackground" : "upscale";
  const Icon = mode === "remove-background" ? Eraser : Maximize2;
  const [inputPath, setInputPath] = useState("");
  const [outputPath, setOutputPath] = useState("");
  const [scale, setScale] = useState<Scale>(4);
  const [format, setFormat] = useState<Format>("png");
  const [queued, setQueued] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function selectInput() {
    setError(null);
    try {
      const selected = await chooseImageFile();
      if (!selected) return;
      setInputPath(selected);
      setOutputPath("");
      setQueued(false);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function selectOutput() {
    if (!inputPath) return;
    setError(null);
    try {
      const selected = await chooseImageOutput(
        outputSuggestion(inputPath, mode, scale, format),
        mode === "remove-background" ? ["png"] : [format],
      );
      if (selected) setOutputPath(selected);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function enqueue() {
    if (!inputPath || !outputPath) return;
    const operation: ImageAiOperation =
      mode === "remove-background"
        ? { type: "remove_background" }
        : { type: "upscale", scale };
    setBusy(true);
    setError(null);
    try {
      await enqueueImageAi(inputPath, outputPath, operation);
      setQueued(true);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }

  const outputStep = mode === "upscale-image" ? 4 : 2;

  return (
    <div className="tool-workspace">
      <button className="back-button" onClick={onBack} type="button">
        <ArrowLeft size={16} />
        {t("imageAi.back")}
      </button>
      <div className="tool-heading">
        <span className="converter-icon">
          <Icon size={20} />
        </span>
        <div>
          <p className="eyebrow accent">{t(`imageAi.${key}.engine`)}</p>
          <h2>{t(`imageAi.${key}.title`)}</h2>
          <p>{t(`imageAi.${key}.description`)}</p>
        </div>
      </div>

      <div className="audio-form">
        <div className="audio-step">
          <span className="step-number">1</span>
          <div>
            <strong>{t("imageAi.input.label")}</strong>
            <p>{t("imageAi.input.help")}</p>
          </div>
          <button
            disabled={!desktopAvailable || busy}
            onClick={() => void selectInput()}
            type="button"
          >
            <FolderOpen size={16} />
            {inputPath ? t("imageAi.input.change") : t("imageAi.input.choose")}
          </button>
          {inputPath && (
            <span className="selected-file" title={inputPath}>
              {fileName(inputPath)}
            </span>
          )}
        </div>

        {mode === "upscale-image" && (
          <>
            <div className="audio-step">
              <span className="step-number">2</span>
              <div>
                <strong>{t("imageAi.scale.label")}</strong>
                <p>{t("imageAi.scale.help")}</p>
              </div>
              <select
                aria-label={t("imageAi.scale.label")}
                disabled={busy}
                onChange={(event) => {
                  setScale(Number(event.target.value) as Scale);
                  setOutputPath("");
                  setQueued(false);
                }}
                value={scale}
              >
                <option value={2}>{t("imageAi.scale.x2")}</option>
                <option value={4}>{t("imageAi.scale.x4")}</option>
              </select>
            </div>
            <div className="audio-step">
              <span className="step-number">3</span>
              <div>
                <strong>{t("imageAi.format.label")}</strong>
                <p>{t("imageAi.format.help")}</p>
              </div>
              <select
                aria-label={t("imageAi.format.label")}
                disabled={busy}
                onChange={(event) => {
                  setFormat(event.target.value as Format);
                  setOutputPath("");
                  setQueued(false);
                }}
                value={format}
              >
                <option value="png">{t("imageAi.format.png")}</option>
                <option value="jpg">{t("imageAi.format.jpg")}</option>
              </select>
            </div>
          </>
        )}

        <div className="audio-step">
          <span className="step-number">{outputStep}</span>
          <div>
            <strong>{t("imageAi.output.label")}</strong>
            <p>{t("imageAi.output.help")}</p>
          </div>
          <button
            disabled={!inputPath || busy}
            onClick={() => void selectOutput()}
            type="button"
          >
            {t("imageAi.output.choose")}
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
      {queued && (
        <div className="success-notice" role="status">
          <span>{t("imageAi.queued")}</span>
          <button onClick={onViewJobs} type="button">
            <ListChecks size={15} />
            {t("imageAi.viewJobs")}
          </button>
        </div>
      )}
      <div className="tool-actions">
        <span>{t("imageAi.localNotice")}</span>
        <button
          className="primary-button"
          disabled={!inputPath || !outputPath || busy}
          onClick={() => void enqueue()}
          type="button"
        >
          {busy ? t("imageAi.adding") : t("imageAi.convert")}
        </button>
      </div>
    </div>
  );
}

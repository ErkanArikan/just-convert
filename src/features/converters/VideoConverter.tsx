import { useState } from "react";
import { ArrowLeft, Clapperboard, FolderOpen, ListChecks } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  chooseDirectory,
  chooseVideoFile,
  chooseVideoOutput,
  enqueueVideoConversion,
  enqueueVideoFrames,
} from "../../services/backend";
import type { VideoFormat, VideoQuality } from "../../types/jobs";

type Task = "convert" | "frames";
type ImageFormat = "png" | "jpeg";

const FORMATS: VideoFormat[] = ["mp4", "webm", "mov", "mkv", "gif"];
const QUALITIES: VideoQuality[] = ["high", "balanced", "small"];
const HEIGHTS = [1080, 720, 480, 360];
const IMAGE_FORMATS: ImageFormat[] = ["png", "jpeg"];

function outputSuggestion(input: string, format: VideoFormat) {
  const withoutExtension = input.replace(/\.[^./\\]+$/, "");
  return `${withoutExtension || input}.${format}`;
}

function fileName(path: string) {
  return path.split(/[\\/]/).pop() ?? path;
}

interface VideoConverterProps {
  desktopAvailable: boolean;
  onBack: () => void;
  onViewJobs: () => void;
}

export function VideoConverter({
  desktopAvailable,
  onBack,
  onViewJobs,
}: VideoConverterProps) {
  const { t } = useTranslation();
  const [task, setTask] = useState<Task>("convert");
  const [inputPath, setInputPath] = useState("");
  const [outputPath, setOutputPath] = useState("");
  const [format, setFormat] = useState<VideoFormat>("mp4");
  const [quality, setQuality] = useState<VideoQuality>("balanced");
  const [maxHeight, setMaxHeight] = useState("original");
  const [interval, setInterval] = useState(1);
  const [imageFormat, setImageFormat] = useState<ImageFormat>("png");
  const [queued, setQueued] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const intervalValid =
    Number.isInteger(interval) && interval >= 1 && interval <= 3600;

  function reset() {
    setOutputPath("");
    setQueued(false);
  }

  async function selectInput() {
    setError(null);
    try {
      const selected = await chooseVideoFile();
      if (!selected) return;
      setInputPath(selected);
      reset();
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function selectOutput() {
    if (!inputPath) return;
    setError(null);
    try {
      const selected =
        task === "convert"
          ? await chooseVideoOutput(outputSuggestion(inputPath, format), format)
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
      if (task === "convert") {
        await enqueueVideoConversion(
          inputPath,
          outputPath,
          format,
          quality,
          maxHeight === "original" ? null : Number(maxHeight),
        );
      } else {
        await enqueueVideoFrames(inputPath, outputPath, imageFormat, interval);
      }
      setQueued(true);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }

  const outputStep = task === "convert" ? 5 : 4;
  const outputKey = task === "convert" ? "video.output" : "video.framesOutput";

  return (
    <div className="tool-workspace">
      <button className="back-button" onClick={onBack} type="button">
        <ArrowLeft size={16} />
        {t("video.back")}
      </button>
      <div className="tool-heading">
        <span className="converter-icon">
          <Clapperboard size={20} />
        </span>
        <div>
          <p className="eyebrow accent">FFmpeg</p>
          <h2>{t("video.title")}</h2>
          <p>{t("video.description")}</p>
        </div>
      </div>

      <div className="audio-form">
        <div className="audio-step">
          <span className="step-number">1</span>
          <div>
            <strong>{t("video.task.label")}</strong>
          </div>
          <select
            aria-label={t("video.task.label")}
            disabled={busy}
            onChange={(event) => {
              setTask(event.target.value as Task);
              reset();
            }}
            value={task}
          >
            <option value="convert">{t("video.task.convert")}</option>
            <option value="frames">{t("video.task.frames")}</option>
          </select>
        </div>

        <div className="audio-step">
          <span className="step-number">2</span>
          <div>
            <strong>{t("video.input.label")}</strong>
            <p>{t("video.input.help")}</p>
          </div>
          <button
            disabled={!desktopAvailable || busy}
            onClick={() => void selectInput()}
            type="button"
          >
            <FolderOpen size={16} />
            {inputPath ? t("video.input.change") : t("video.input.choose")}
          </button>
          {inputPath && (
            <span className="selected-file" title={inputPath}>
              {fileName(inputPath)}
            </span>
          )}
        </div>

        {task === "convert" ? (
          <>
            <div className="audio-step">
              <span className="step-number">3</span>
              <div>
                <strong>{t("video.format.label")}</strong>
                <p>{t("video.format.help")}</p>
              </div>
              <select
                aria-label={t("video.format.label")}
                disabled={busy}
                onChange={(event) => {
                  setFormat(event.target.value as VideoFormat);
                  reset();
                }}
                value={format}
              >
                {FORMATS.map((item) => (
                  <option key={item} value={item}>
                    {t(`video.format.${item}`)}
                  </option>
                ))}
              </select>
            </div>

            <div className="audio-step">
              <span className="step-number">4</span>
              <div>
                <strong>{t("video.quality.label")}</strong>
                <p>{t("video.quality.help")}</p>
              </div>
              <select
                aria-label={t("video.quality.label")}
                disabled={busy}
                onChange={(event) => {
                  setQuality(event.target.value as VideoQuality);
                  setQueued(false);
                }}
                value={quality}
              >
                {QUALITIES.map((item) => (
                  <option key={item} value={item}>
                    {t(`video.quality.${item}`)}
                  </option>
                ))}
              </select>
              <select
                aria-label={t("video.height.label")}
                disabled={busy}
                onChange={(event) => {
                  setMaxHeight(event.target.value);
                  setQueued(false);
                }}
                title={t("video.height.help")}
                value={maxHeight}
              >
                <option value="original">{t("video.height.original")}</option>
                {HEIGHTS.map((height) => (
                  <option key={height} value={height}>
                    {height}p
                  </option>
                ))}
              </select>
            </div>
          </>
        ) : (
          <div className="audio-step">
            <span className="step-number">3</span>
            <div>
              <strong>{t("video.interval.label")}</strong>
              <p>{t("video.interval.help")}</p>
            </div>
            <input
              aria-label={t("video.interval.label")}
              disabled={busy}
              max={3600}
              min={1}
              onChange={(event) => {
                setInterval(event.target.valueAsNumber);
                setQueued(false);
              }}
              step={1}
              type="number"
              value={Number.isNaN(interval) ? "" : interval}
            />
            <select
              aria-label={t("video.imageFormat.label")}
              disabled={busy}
              onChange={(event) => {
                setImageFormat(event.target.value as ImageFormat);
                setQueued(false);
              }}
              title={t("video.imageFormat.help")}
              value={imageFormat}
            >
              {IMAGE_FORMATS.map((item) => (
                <option key={item} value={item}>
                  {t(`video.imageFormat.${item}`)}
                </option>
              ))}
            </select>
          </div>
        )}

        <div className="audio-step">
          <span className="step-number">{outputStep}</span>
          <div>
            <strong>{t(`${outputKey}.label`)}</strong>
            <p>{t(`${outputKey}.help`)}</p>
          </div>
          <button
            disabled={!inputPath || busy}
            onClick={() => void selectOutput()}
            type="button"
          >
            {t(`${outputKey}.choose`)}
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
          <span>{t("video.queued")}</span>
          <button onClick={onViewJobs} type="button">
            <ListChecks size={15} />
            {t("video.viewJobs")}
          </button>
        </div>
      )}
      <div className="tool-actions">
        <span>{t("video.localNotice")}</span>
        <button
          className="primary-button"
          disabled={
            !inputPath ||
            !outputPath ||
            busy ||
            (task === "frames" && !intervalValid)
          }
          onClick={() => void enqueue()}
          type="button"
        >
          {busy ? t("video.adding") : t("video.convert")}
        </button>
      </div>
    </div>
  );
}

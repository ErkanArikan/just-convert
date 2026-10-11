import { useState } from "react";
import { ArrowLeft, FileText, ListChecks, Volume2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  chooseAudioOutput,
  chooseTextFile,
  enqueueTextToSpeech,
  readTextFile,
} from "../../services/backend";
import type { AudioFormat, TtsSpeed, TtsVoice } from "../../types/jobs";

const MAX_CHARACTERS = 20000;
const FORMATS: AudioFormat[] = ["mp3", "wav", "flac", "aac", "m4a", "ogg"];
const SPEEDS: TtsSpeed[] = ["slow", "normal", "fast"];

function fileName(path: string) {
  return path.split(/[\\/]/).pop() ?? path;
}

interface TextToSpeechProps {
  desktopAvailable: boolean;
  onBack: () => void;
  onViewJobs: () => void;
}

export function TextToSpeech({
  desktopAvailable,
  onBack,
  onViewJobs,
}: TextToSpeechProps) {
  const { t } = useTranslation();
  const [text, setText] = useState("");
  const [sourceName, setSourceName] = useState("speech");
  const [outputPath, setOutputPath] = useState("");
  const [voice, setVoice] = useState<TtsVoice>("english");
  const [speed, setSpeed] = useState<TtsSpeed>("normal");
  const [format, setFormat] = useState<AudioFormat>("mp3");
  const [queued, setQueued] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const length = Array.from(text).length;
  const textReady = text.trim().length > 0 && length <= MAX_CHARACTERS;

  async function openTextFile() {
    setError(null);
    try {
      const selected = await chooseTextFile();
      if (!selected) return;
      setText(await readTextFile(selected));
      setSourceName(fileName(selected).replace(/\.[^.]+$/, "") || "speech");
      setQueued(false);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function selectOutput() {
    setError(null);
    try {
      const selected = await chooseAudioOutput(
        `${sourceName}.${format}`,
        format,
      );
      if (selected) setOutputPath(selected);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function enqueue() {
    if (!textReady || !outputPath) return;
    setBusy(true);
    setError(null);
    try {
      await enqueueTextToSpeech(text, outputPath, format, voice, speed);
      setQueued(true);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="tool-workspace">
      <button className="back-button" onClick={onBack} type="button">
        <ArrowLeft size={16} />
        {t("tts.back")}
      </button>
      <div className="tool-heading">
        <span className="converter-icon">
          <Volume2 size={20} />
        </span>
        <div>
          <p className="eyebrow accent">Piper</p>
          <h2>{t("tts.title")}</h2>
          <p>{t("tts.description")}</p>
        </div>
      </div>

      <div className="audio-form">
        <div className="audio-step">
          <span className="step-number">1</span>
          <div>
            <strong>{t("tts.text.label")}</strong>
            <p>{t("tts.text.help")}</p>
          </div>
          <button
            disabled={!desktopAvailable || busy}
            onClick={() => void openTextFile()}
            type="button"
          >
            <FileText size={16} />
            {t("tts.text.open")}
          </button>
        </div>
        <div className="tts-text">
          <textarea
            aria-label={t("tts.text.label")}
            disabled={busy}
            onChange={(event) => {
              setText(event.target.value);
              setQueued(false);
            }}
            placeholder={t("tts.text.placeholder")}
            rows={7}
            value={text}
          />
          <p
            aria-live="polite"
            className={length > MAX_CHARACTERS ? "inline-error" : undefined}
          >
            {t("tts.text.count", { count: length, max: MAX_CHARACTERS })}
          </p>
        </div>

        <div className="audio-step">
          <span className="step-number">2</span>
          <div>
            <strong>{t("tts.voice.label")}</strong>
            <p>{t("tts.voice.help")}</p>
          </div>
          <select
            aria-label={t("tts.voice.label")}
            disabled={busy}
            onChange={(event) => {
              setVoice(event.target.value as TtsVoice);
              setQueued(false);
            }}
            value={voice}
          >
            <option value="english">{t("tts.voice.english")}</option>
            <option value="turkish">{t("tts.voice.turkish")}</option>
          </select>
        </div>

        <div className="audio-step">
          <span className="step-number">3</span>
          <div>
            <strong>{t("tts.speed.label")}</strong>
            <p>{t("tts.speed.help")}</p>
          </div>
          <select
            aria-label={t("tts.speed.label")}
            disabled={busy}
            onChange={(event) => {
              setSpeed(event.target.value as TtsSpeed);
              setQueued(false);
            }}
            value={speed}
          >
            {SPEEDS.map((item) => (
              <option key={item} value={item}>
                {t(`tts.speed.${item}`)}
              </option>
            ))}
          </select>
        </div>

        <div className="audio-step">
          <span className="step-number">4</span>
          <div>
            <strong>{t("tts.format.label")}</strong>
            <p>{t("tts.format.help")}</p>
          </div>
          <select
            aria-label={t("tts.format.label")}
            disabled={busy}
            onChange={(event) => {
              setFormat(event.target.value as AudioFormat);
              setOutputPath("");
              setQueued(false);
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
          <span className="step-number">5</span>
          <div>
            <strong>{t("tts.output.label")}</strong>
            <p>{t("tts.output.help")}</p>
          </div>
          <button
            disabled={!desktopAvailable || busy}
            onClick={() => void selectOutput()}
            type="button"
          >
            {t("tts.output.choose")}
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
          <span>{t("tts.queued")}</span>
          <button onClick={onViewJobs} type="button">
            <ListChecks size={15} />
            {t("tts.viewJobs")}
          </button>
        </div>
      )}
      <div className="tool-actions">
        <span>{t("tts.localNotice")}</span>
        <button
          className="primary-button"
          disabled={!textReady || !outputPath || busy}
          onClick={() => void enqueue()}
          type="button"
        >
          {busy ? t("tts.adding") : t("tts.convert")}
        </button>
      </div>
    </div>
  );
}

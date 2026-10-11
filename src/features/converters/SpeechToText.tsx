import { useMemo, useState } from "react";
import { ArrowLeft, Captions, FolderOpen, ListChecks } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  chooseAudioFile,
  chooseTranscriptOutput,
  enqueueTranscription,
} from "../../services/backend";
import type { SpeechModel, TranscriptFormat } from "../../types/jobs";

const FORMATS: TranscriptFormat[] = ["txt", "srt", "vtt", "json"];
const LANGUAGES = [
  "en",
  "tr",
  "de",
  "fr",
  "es",
  "it",
  "pt",
  "nl",
  "pl",
  "ru",
  "uk",
  "ar",
  "fa",
  "hi",
  "zh",
  "ja",
  "ko",
];

function outputSuggestion(input: string, format: TranscriptFormat) {
  const withoutExtension = input.replace(/\.[^./\\]+$/, "");
  return `${withoutExtension || input}.${format}`;
}

function fileName(path: string) {
  return path.split(/[\\/]/).pop() ?? path;
}

interface SpeechToTextProps {
  desktopAvailable: boolean;
  onBack: () => void;
  onViewJobs: () => void;
}

export function SpeechToText({
  desktopAvailable,
  onBack,
  onViewJobs,
}: SpeechToTextProps) {
  const { t, i18n } = useTranslation();
  const [inputPath, setInputPath] = useState("");
  const [outputPath, setOutputPath] = useState("");
  const [format, setFormat] = useState<TranscriptFormat>("txt");
  const [language, setLanguage] = useState("auto");
  const [model, setModel] = useState<SpeechModel>("standard");
  const [queued, setQueued] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const languageNames = useMemo(() => {
    const names = new Intl.DisplayNames([i18n.language], { type: "language" });
    return LANGUAGES.map((code) => ({ code, name: names.of(code) ?? code }));
  }, [i18n.language]);

  async function selectInput() {
    setError(null);
    try {
      const selected = await chooseAudioFile();
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
      const selected = await chooseTranscriptOutput(
        outputSuggestion(inputPath, format),
        format,
      );
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
      await enqueueTranscription(
        inputPath,
        outputPath,
        format,
        language === "auto" ? null : language,
        model,
      );
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
        {t("speech.back")}
      </button>
      <div className="tool-heading">
        <span className="converter-icon">
          <Captions size={20} />
        </span>
        <div>
          <p className="eyebrow accent">Whisper</p>
          <h2>{t("speech.title")}</h2>
          <p>{t("speech.description")}</p>
        </div>
      </div>

      <div className="audio-form">
        <div className="audio-step">
          <span className="step-number">1</span>
          <div>
            <strong>{t("speech.input.label")}</strong>
            <p>{t("speech.input.help")}</p>
          </div>
          <button
            disabled={!desktopAvailable || busy}
            onClick={() => void selectInput()}
            type="button"
          >
            <FolderOpen size={16} />
            {inputPath ? t("speech.input.change") : t("speech.input.choose")}
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
            <strong>{t("speech.format.label")}</strong>
            <p>{t("speech.format.help")}</p>
          </div>
          <select
            aria-label={t("speech.format.label")}
            disabled={busy}
            onChange={(event) => {
              setFormat(event.target.value as TranscriptFormat);
              setOutputPath("");
              setQueued(false);
            }}
            value={format}
          >
            {FORMATS.map((item) => (
              <option key={item} value={item}>
                {t(`speech.format.${item}`)}
              </option>
            ))}
          </select>
        </div>

        <div className="audio-step">
          <span className="step-number">3</span>
          <div>
            <strong>{t("speech.language.label")}</strong>
            <p>{t("speech.language.help")}</p>
          </div>
          <select
            aria-label={t("speech.language.label")}
            disabled={busy}
            onChange={(event) => {
              setLanguage(event.target.value);
              setQueued(false);
            }}
            value={language}
          >
            <option value="auto">{t("speech.language.auto")}</option>
            {languageNames.map(({ code, name }) => (
              <option key={code} value={code}>
                {name}
              </option>
            ))}
          </select>
        </div>

        <div className="audio-step">
          <span className="step-number">4</span>
          <div>
            <strong>{t("speech.model.label")}</strong>
            <p>{t("speech.model.help")}</p>
          </div>
          <select
            aria-label={t("speech.model.label")}
            disabled={busy}
            onChange={(event) => {
              setModel(event.target.value as SpeechModel);
              setQueued(false);
            }}
            value={model}
          >
            <option value="standard">{t("speech.model.standard")}</option>
            <option value="high_accuracy">
              {t("speech.model.highAccuracy")}
            </option>
          </select>
        </div>

        <div className="audio-step">
          <span className="step-number">5</span>
          <div>
            <strong>{t("speech.output.label")}</strong>
            <p>{t("speech.output.help")}</p>
          </div>
          <button
            disabled={!inputPath || busy}
            onClick={() => void selectOutput()}
            type="button"
          >
            {t("speech.output.choose")}
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
          <span>{t("speech.queued")}</span>
          <button onClick={onViewJobs} type="button">
            <ListChecks size={15} />
            {t("speech.viewJobs")}
          </button>
        </div>
      )}
      <div className="tool-actions">
        <span>{t("speech.localNotice")}</span>
        <button
          className="primary-button"
          disabled={!inputPath || !outputPath || busy}
          onClick={() => void enqueue()}
          type="button"
        >
          {busy ? t("speech.adding") : t("speech.convert")}
        </button>
      </div>
    </div>
  );
}

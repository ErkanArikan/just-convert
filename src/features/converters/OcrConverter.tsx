import { useState } from "react";
import { ArrowLeft, FolderOpen, ListChecks, ScanText } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  chooseOcrFile,
  chooseOcrOutput,
  enqueueOcr,
} from "../../services/backend";
import type { OcrFormat, OcrLanguage } from "../../types/jobs";

type LanguageChoice = "eng" | "tur" | "both";

const LANGUAGES: Record<LanguageChoice, OcrLanguage[]> = {
  eng: ["eng"],
  tur: ["tur"],
  both: ["eng", "tur"],
};

function outputSuggestion(input: string, format: OcrFormat) {
  const withoutExtension = input.replace(/\.[^./\\]+$/, "");
  const suffix = format === "pdf" && /\.pdf$/i.test(input) ? "-ocr" : "";
  return `${withoutExtension || input}${suffix}.${format}`;
}

function fileName(path: string) {
  return path.split(/[\\/]/).pop() ?? path;
}

interface OcrConverterProps {
  desktopAvailable: boolean;
  onBack: () => void;
  onViewJobs: () => void;
}

export function OcrConverter({
  desktopAvailable,
  onBack,
  onViewJobs,
}: OcrConverterProps) {
  const { t } = useTranslation();
  const [inputPath, setInputPath] = useState("");
  const [outputPath, setOutputPath] = useState("");
  const [language, setLanguage] = useState<LanguageChoice>("eng");
  const [format, setFormat] = useState<OcrFormat>("pdf");
  const [queued, setQueued] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function selectInput() {
    setError(null);
    try {
      const selected = await chooseOcrFile();
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
      const selected = await chooseOcrOutput(
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
      await enqueueOcr(inputPath, outputPath, format, LANGUAGES[language]);
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
        {t("ocr.back")}
      </button>
      <div className="tool-heading">
        <span className="converter-icon">
          <ScanText size={20} />
        </span>
        <div>
          <p className="eyebrow accent">Tesseract</p>
          <h2>{t("ocr.title")}</h2>
          <p>{t("ocr.description")}</p>
        </div>
      </div>

      <div className="audio-form">
        <div className="audio-step">
          <span className="step-number">1</span>
          <div>
            <strong>{t("ocr.input.label")}</strong>
            <p>{t("ocr.input.help")}</p>
          </div>
          <button
            disabled={!desktopAvailable || busy}
            onClick={() => void selectInput()}
            type="button"
          >
            <FolderOpen size={16} />
            {inputPath ? t("ocr.input.change") : t("ocr.input.choose")}
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
            <strong>{t("ocr.language.label")}</strong>
            <p>{t("ocr.language.help")}</p>
          </div>
          <select
            aria-label={t("ocr.language.label")}
            disabled={busy}
            onChange={(event) => {
              setLanguage(event.target.value as LanguageChoice);
              setQueued(false);
            }}
            value={language}
          >
            {(Object.keys(LANGUAGES) as LanguageChoice[]).map((item) => (
              <option key={item} value={item}>
                {t(`ocr.language.${item}`)}
              </option>
            ))}
          </select>
        </div>

        <div className="audio-step">
          <span className="step-number">3</span>
          <div>
            <strong>{t("ocr.format.label")}</strong>
            <p>{t("ocr.format.help")}</p>
          </div>
          <select
            aria-label={t("ocr.format.label")}
            disabled={busy}
            onChange={(event) => {
              setFormat(event.target.value as OcrFormat);
              setOutputPath("");
              setQueued(false);
            }}
            value={format}
          >
            <option value="pdf">{t("ocr.format.pdf")}</option>
            <option value="txt">{t("ocr.format.txt")}</option>
          </select>
        </div>

        <div className="audio-step">
          <span className="step-number">4</span>
          <div>
            <strong>{t("ocr.output.label")}</strong>
            <p>{t("ocr.output.help")}</p>
          </div>
          <button
            disabled={!inputPath || busy}
            onClick={() => void selectOutput()}
            type="button"
          >
            {t("ocr.output.choose")}
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
          <span>{t("ocr.queued")}</span>
          <button onClick={onViewJobs} type="button">
            <ListChecks size={15} />
            {t("ocr.viewJobs")}
          </button>
        </div>
      )}
      <div className="tool-actions">
        <span>{t("ocr.localNotice")}</span>
        <button
          className="primary-button"
          disabled={!inputPath || !outputPath || busy}
          onClick={() => void enqueue()}
          type="button"
        >
          {busy ? t("ocr.adding") : t("ocr.convert")}
        </button>
      </div>
    </div>
  );
}

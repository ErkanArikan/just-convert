import { useState } from "react";
import {
  ArrowDown,
  ArrowLeft,
  ArrowUp,
  FilePlus2,
  FolderOpen,
  ListChecks,
  RotateCw,
  Scissors,
  Shuffle,
  Trash2,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  chooseDirectory,
  choosePdfFiles,
  choosePdfOutput,
  enqueuePdfOperation,
  getPdfPageCount,
} from "../../services/backend";
import type { ConversionJob, PdfOperation } from "../../types/jobs";

type PdfMode = "merge" | "split" | "reorder" | "rotate";

const modes = [
  { id: "merge", icon: FilePlus2 },
  { id: "split", icon: Scissors },
  { id: "reorder", icon: Shuffle },
  { id: "rotate", icon: RotateCw },
] as const;

function fileName(path: string) {
  return path.split(/[\\/]/).pop() ?? path;
}

function outputSuggestion(input: string, suffix: string) {
  const withoutExtension = input.replace(/\.[^./\\]+$/, "");
  return `${withoutExtension || input}-${suffix}.pdf`;
}

function parsePages(value: string, allowEmpty: boolean) {
  const normalized = value.trim();
  if (!normalized && allowEmpty) return [];
  if (!normalized) throw new Error("page_list_required");
  const pages: number[] = [];
  for (const token of normalized.split(/[\s,]+/).filter(Boolean)) {
    const range = token.match(/^(\d+)-(\d+)$/);
    if (range) {
      const start = Number(range[1]);
      const end = Number(range[2]);
      if (start < 1 || end < start || end - start > 10_000)
        throw new Error("invalid_page_list");
      for (let page = start; page <= end; page += 1) pages.push(page);
    } else if (/^\d+$/.test(token) && Number(token) > 0) {
      pages.push(Number(token));
    } else {
      throw new Error("invalid_page_list");
    }
  }
  if (new Set(pages).size !== pages.length) throw new Error("duplicate_pages");
  return pages;
}

interface PdfToolsProps {
  desktopAvailable: boolean;
  onBack: () => void;
  onViewJobs: () => void;
}

export function PdfTools({
  desktopAvailable,
  onBack,
  onViewJobs,
}: PdfToolsProps) {
  const { t } = useTranslation();
  const [mode, setMode] = useState<PdfMode>("merge");
  const [inputs, setInputs] = useState<string[]>([]);
  const [inputPath, setInputPath] = useState("");
  const [pageCount, setPageCount] = useState<number | null>(null);
  const [outputPath, setOutputPath] = useState("");
  const [outputDirectory, setOutputDirectory] = useState("");
  const [pagesPerFile, setPagesPerFile] = useState(1);
  const [pageOrder, setPageOrder] = useState("");
  const [angle, setAngle] = useState(90);
  const [rotatePages, setRotatePages] = useState("");
  const [queuedJob, setQueuedJob] = useState<ConversionJob | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function changeMode(next: PdfMode) {
    setMode(next);
    setInputs([]);
    setInputPath("");
    setPageCount(null);
    setOutputPath("");
    setOutputDirectory("");
    setQueuedJob(null);
    setError(null);
  }

  async function selectMergeInputs() {
    setError(null);
    try {
      const selected = await choosePdfFiles(true);
      setInputs((current) => [...new Set([...current, ...selected])]);
      setOutputPath("");
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function selectSingleInput() {
    setError(null);
    setBusy(true);
    try {
      const [selected] = await choosePdfFiles(false);
      if (!selected) return;
      setInputPath(selected);
      setOutputPath("");
      setOutputDirectory("");
      setPageCount(await getPdfPageCount(selected));
    } catch (reason) {
      setError(String(reason));
      setInputPath("");
      setPageCount(null);
    } finally {
      setBusy(false);
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
  }

  async function selectOutput() {
    const source = mode === "merge" ? inputs[0] : inputPath;
    if (!source) return;
    try {
      const selected = await choosePdfOutput(outputSuggestion(source, mode));
      if (selected) setOutputPath(selected);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function selectSplitDirectory() {
    try {
      const selected = await chooseDirectory();
      if (selected) setOutputDirectory(selected);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function enqueue() {
    setBusy(true);
    setError(null);
    try {
      let operation: PdfOperation;
      if (mode === "merge") {
        operation = { type: "merge", inputPaths: inputs, outputPath };
      } else if (mode === "split") {
        operation = {
          type: "split",
          inputPath,
          outputDirectory,
          pagesPerFile,
        };
      } else if (mode === "reorder") {
        operation = {
          type: "reorder",
          inputPath,
          outputPath,
          pageOrder: parsePages(pageOrder, false),
        };
      } else {
        operation = {
          type: "rotate",
          inputPath,
          outputPath,
          angle,
          pages: parsePages(rotatePages, true),
        };
      }
      setQueuedJob(await enqueuePdfOperation(operation));
    } catch (reason) {
      const message = reason instanceof Error ? reason.message : String(reason);
      setError(
        ["page_list_required", "invalid_page_list", "duplicate_pages"].includes(
          message,
        )
          ? t(`pdf.errors.${message}`)
          : message,
      );
    } finally {
      setBusy(false);
    }
  }

  const ready =
    mode === "merge"
      ? inputs.length >= 2 && Boolean(outputPath)
      : mode === "split"
        ? Boolean(inputPath && outputDirectory)
        : Boolean(inputPath && outputPath);

  return (
    <div className="tool-workspace">
      <button className="back-button" onClick={onBack} type="button">
        <ArrowLeft size={16} />
        {t("pdf.back")}
      </button>
      <div className="tool-heading">
        <span className="converter-icon">
          <FilePlus2 size={20} />
        </span>
        <div>
          <p className="eyebrow accent">qpdf</p>
          <h2>{t("pdf.title")}</h2>
          <p>{t("pdf.description")}</p>
        </div>
      </div>

      <div
        className="pdf-mode-tabs"
        role="tablist"
        aria-label={t("pdf.modeLabel")}
      >
        {modes.map(({ id, icon: Icon }) => (
          <button
            aria-selected={mode === id}
            data-active={mode === id}
            key={id}
            onClick={() => changeMode(id)}
            role="tab"
            type="button"
          >
            <Icon size={15} />
            {t(`pdf.modes.${id}`)}
          </button>
        ))}
      </div>

      <div className="pdf-form">
        {mode === "merge" ? (
          <>
            <div className="pdf-form-heading">
              <div>
                <strong>{t("pdf.merge.files")}</strong>
                <p>{t("pdf.merge.help")}</p>
              </div>
              <button
                disabled={!desktopAvailable || busy}
                onClick={() => void selectMergeInputs()}
                type="button"
              >
                <FolderOpen size={15} />
                {t("pdf.choosePdfs")}
              </button>
            </div>
            <div className="pdf-input-list">
              {inputs.length ? (
                inputs.map((path, index) => (
                  <div className="pdf-input-row" key={path}>
                    <span className="pdf-order">{index + 1}</span>
                    <span title={path}>{fileName(path)}</span>
                    <button
                      aria-label={t("pdf.moveUp", { name: fileName(path) })}
                      disabled={index === 0}
                      onClick={() => moveInput(index, -1)}
                      type="button"
                    >
                      <ArrowUp size={14} />
                    </button>
                    <button
                      aria-label={t("pdf.moveDown", { name: fileName(path) })}
                      disabled={index === inputs.length - 1}
                      onClick={() => moveInput(index, 1)}
                      type="button"
                    >
                      <ArrowDown size={14} />
                    </button>
                    <button
                      aria-label={t("pdf.remove", { name: fileName(path) })}
                      onClick={() =>
                        setInputs((current) =>
                          current.filter((item) => item !== path),
                        )
                      }
                      type="button"
                    >
                      <Trash2 size={14} />
                    </button>
                  </div>
                ))
              ) : (
                <span className="pdf-list-empty">{t("pdf.merge.empty")}</span>
              )}
            </div>
          </>
        ) : (
          <div className="pdf-form-heading">
            <div>
              <strong>{t("pdf.source")}</strong>
              <p>
                {inputPath
                  ? t("pdf.pageCount", { count: pageCount ?? "…" })
                  : t("pdf.sourceHelp")}
              </p>
            </div>
            <button
              disabled={!desktopAvailable || busy}
              onClick={() => void selectSingleInput()}
              type="button"
            >
              <FolderOpen size={15} />
              {inputPath ? fileName(inputPath) : t("pdf.choosePdf")}
            </button>
          </div>
        )}

        {mode === "split" && (
          <div className="pdf-options-grid">
            <label>
              <span>{t("pdf.split.pagesPerFile")}</span>
              <input
                max={1000}
                min={1}
                onChange={(event) =>
                  setPagesPerFile(Number(event.target.value))
                }
                type="number"
                value={pagesPerFile}
              />
            </label>
            <label>
              <span>{t("pdf.split.outputDirectory")}</span>
              <button
                disabled={!inputPath || busy}
                onClick={() => void selectSplitDirectory()}
                type="button"
              >
                {outputDirectory
                  ? fileName(outputDirectory)
                  : t("pdf.chooseFolder")}
              </button>
            </label>
          </div>
        )}

        {mode === "reorder" && (
          <div className="pdf-options-grid">
            <label className="wide-option">
              <span>{t("pdf.reorder.order")}</span>
              <input
                onChange={(event) => setPageOrder(event.target.value)}
                placeholder={t("pdf.reorder.placeholder")}
                type="text"
                value={pageOrder}
              />
              <small>{t("pdf.reorder.help")}</small>
            </label>
          </div>
        )}

        {mode === "rotate" && (
          <div className="pdf-options-grid">
            <label>
              <span>{t("pdf.rotate.angle")}</span>
              <select
                onChange={(event) => setAngle(Number(event.target.value))}
                value={angle}
              >
                <option value={90}>90°</option>
                <option value={180}>180°</option>
                <option value={270}>270°</option>
              </select>
            </label>
            <label>
              <span>{t("pdf.rotate.pages")}</span>
              <input
                onChange={(event) => setRotatePages(event.target.value)}
                placeholder={t("pdf.rotate.placeholder")}
                type="text"
                value={rotatePages}
              />
              <small>{t("pdf.rotate.help")}</small>
            </label>
          </div>
        )}

        {mode !== "split" && (
          <div className="pdf-output-row">
            <div>
              <strong>{t("pdf.output")}</strong>
              <p>{outputPath ? fileName(outputPath) : t("pdf.outputHelp")}</p>
            </div>
            <button
              disabled={!(mode === "merge" ? inputs.length : inputPath) || busy}
              onClick={() => void selectOutput()}
              type="button"
            >
              {t("pdf.chooseOutput")}
            </button>
          </div>
        )}
      </div>

      {error && (
        <div className="inline-error" role="alert">
          {error}
        </div>
      )}
      {queuedJob && (
        <div className="success-notice" role="status">
          <span>{t("pdf.queued")}</span>
          <button onClick={onViewJobs} type="button">
            <ListChecks size={15} />
            {t("pdf.viewJobs")}
          </button>
        </div>
      )}
      <div className="tool-actions">
        <span>{t("pdf.localNotice")}</span>
        <button
          className="primary-button"
          disabled={!ready || busy}
          onClick={() => void enqueue()}
          type="button"
        >
          {busy ? t("pdf.adding") : t("pdf.addToQueue")}
        </button>
      </div>
    </div>
  );
}

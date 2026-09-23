import { useCallback, useEffect, useState } from "react";
import {
  Ban,
  Check,
  ChevronDown,
  ChevronRight,
  FileAudio,
  LoaderCircle,
  RefreshCw,
  Trash2,
  X,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  cancelJob,
  clearJobHistory,
  clearJobHistorySection,
  deleteJobHistory,
  listJobs,
} from "../../services/backend";
import type { ConversionJob } from "../../types/jobs";
import {
  usePreferencesStore,
  type JobSectionId,
} from "../../store/preferences";

function fileName(path: string) {
  return path.split(/[\\/]/).pop() ?? path;
}

interface JobListProps {
  desktopAvailable: boolean;
}

export function JobList({ desktopAvailable }: JobListProps) {
  const { t, i18n } = useTranslation();
  const [jobs, setJobs] = useState<ConversionJob[]>([]);
  const [error, setError] = useState<string | null>(null);
  const sectionPreferences = usePreferencesStore((state) => state.jobSections);
  const setJobSectionExpanded = usePreferencesStore(
    (state) => state.setJobSectionExpanded,
  );

  const refresh = useCallback(async () => {
    if (!desktopAvailable) return;
    try {
      setJobs(await listJobs());
      setError(null);
    } catch (reason) {
      setError(String(reason));
    }
  }, [desktopAvailable]);

  useEffect(() => {
    if (!desktopAvailable) return;
    const initialRefresh = window.setTimeout(() => void refresh(), 0);
    const timer = window.setInterval(() => void refresh(), 750);
    return () => {
      window.clearTimeout(initialRefresh);
      window.clearInterval(timer);
    };
  }, [desktopAvailable, refresh]);

  async function cancel(id: string) {
    try {
      await cancelJob(id);
      await refresh();
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function removeHistoryEntry(id: string) {
    try {
      await deleteJobHistory(id);
      await refresh();
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function clearHistory() {
    if (!window.confirm(t("jobs.clearConfirmation"))) return;
    try {
      await clearJobHistory();
      await refresh();
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function clearSection(section: "completed" | "failed") {
    if (!window.confirm(t(`jobs.sections.${section}.clearConfirmation`)))
      return;
    try {
      await clearJobHistorySection(section);
      await refresh();
    } catch (reason) {
      setError(String(reason));
    }
  }

  function renderJob(job: ConversionJob) {
    const summary = (() => {
      if (job.kind.type === "audio_conversion") {
        return {
          output: fileName(job.kind.output_path),
          detail: `${fileName(job.kind.input_path)} → ${job.kind.format.toUpperCase()}`,
        };
      }
      if (job.kind.type === "images_to_pdf") {
        return {
          output: fileName(job.kind.output_path),
          detail: t("jobs.imagesToPdfDetail", {
            count: job.kind.input_paths.length,
          }),
        };
      }
      if (job.kind.type === "pdf_to_images") {
        return {
          output: fileName(job.kind.output_directory),
          detail: t("jobs.pdfToImagesDetail", {
            source: fileName(job.kind.input_path),
            format: job.kind.format.toUpperCase(),
            dpi: job.kind.dpi,
          }),
        };
      }
      if (job.kind.type === "office_to_pdf") {
        return {
          output: fileName(job.kind.output_path),
          detail: t("jobs.officeToPdfDetail", {
            source: fileName(job.kind.input_path),
          }),
        };
      }
      if (job.kind.type === "pdf_to_docx") {
        return {
          output: fileName(job.kind.output_path),
          detail: t("jobs.pdfToDocxDetail", {
            source: fileName(job.kind.input_path),
          }),
        };
      }
      const operation = job.kind.operation;
      if (operation.type === "merge") {
        return {
          output: fileName(operation.outputPath),
          detail: t("jobs.pdfMergeDetail", {
            count: operation.inputPaths.length,
          }),
        };
      }
      if (operation.type === "split") {
        return {
          output: fileName(operation.outputDirectory),
          detail: t("jobs.pdfDetail", {
            source: fileName(operation.inputPath),
            operation: t("pdf.modes.split"),
          }),
        };
      }
      return {
        output: fileName(operation.outputPath),
        detail: t("jobs.pdfDetail", {
          source: fileName(operation.inputPath),
          operation: t(`pdf.modes.${operation.type}`),
        }),
      };
    })();
    const active = job.status === "queued" || job.status === "running";
    const StatusIcon =
      job.status === "completed"
        ? Check
        : job.status === "failed"
          ? X
          : job.status === "cancelled"
            ? Ban
            : LoaderCircle;
    const reason =
      job.status === "cancelled"
        ? t("jobs.cancelledReason")
        : job.status === "failed"
          ? (job.error?.message ?? t("jobs.failedReason"))
          : null;

    return (
      <article className="job-card" data-active={active} key={job.id}>
        <span className="job-status-icon" data-status={job.status}>
          <StatusIcon className={active ? "spin" : ""} size={17} />
        </span>
        <div className="job-details">
          <div className="job-title-row">
            <strong>{summary.output}</strong>
            <span className="status-pill" data-status={job.status}>
              {t(`jobs.status.${job.status}`)}
            </span>
          </div>
          <p>{summary.detail}</p>
          {active && (
            <div className="progress-track" aria-label={t("jobs.progress")}>
              <span style={{ width: `${job.progress}%` }} />
            </div>
          )}
          <small>
            {new Intl.DateTimeFormat(i18n.language, {
              dateStyle: "medium",
              timeStyle: "short",
            }).format(new Date(job.createdAt * 1000))}
          </small>
          {reason && <div className="job-error">{reason}</div>}
        </div>
        {active ? (
          <button
            className="cancel-job"
            onClick={() => void cancel(job.id)}
            type="button"
          >
            {t("jobs.cancel")}
          </button>
        ) : (
          <button
            aria-label={t("jobs.deleteEntryLabel", { name: summary.output })}
            className="delete-job"
            onClick={() => void removeHistoryEntry(job.id)}
            title={t("jobs.deleteEntry")}
            type="button"
          >
            <Trash2 size={16} />
          </button>
        )}
      </article>
    );
  }

  if (!jobs.length && !error) {
    return (
      <div className="file-empty-state jobs-empty">
        <span className="empty-icon">
          <FileAudio size={24} />
        </span>
        <h2>{t("jobs.empty.title")}</h2>
        <p>
          {desktopAvailable
            ? t("jobs.empty.description")
            : t("jobs.empty.browserDescription")}
        </p>
      </div>
    );
  }

  const sections = [
    {
      id: "active" as const,
      jobs: jobs.filter((job) => ["queued", "running"].includes(job.status)),
    },
    {
      id: "failed" as const,
      jobs: jobs.filter((job) => ["failed", "cancelled"].includes(job.status)),
    },
    {
      id: "completed" as const,
      jobs: jobs.filter((job) => job.status === "completed"),
    },
  ];

  function sectionIsExpanded(section: JobSectionId) {
    if (section === "active" && jobs.some((job) => job.status === "running")) {
      return true;
    }
    return sectionPreferences[section];
  }

  function toggleSection(section: JobSectionId) {
    if (section === "active" && jobs.some((job) => job.status === "running")) {
      return;
    }
    setJobSectionExpanded(section, !sectionIsExpanded(section));
  }

  return (
    <div className="jobs-workspace">
      <div className="jobs-heading">
        <div>
          <p className="eyebrow">{t("jobs.eyebrow")}</p>
          <h2>{t("jobs.title")}</h2>
        </div>
        <div className="jobs-heading-actions">
          <button onClick={() => void refresh()} type="button">
            <RefreshCw size={15} />
            {t("jobs.refresh")}
          </button>
          <button
            className="clear-jobs"
            disabled={
              !jobs.some((job) =>
                ["completed", "cancelled"].includes(job.status),
              )
            }
            onClick={() => void clearHistory()}
            type="button"
          >
            <Trash2 size={15} />
            {t("jobs.clearAll")}
          </button>
        </div>
      </div>
      {error && (
        <div className="inline-error" role="alert">
          {error}
        </div>
      )}
      <div className="job-sections">
        {sections.map(
          (section) =>
            section.jobs.length > 0 && (
              <section className="job-section" key={section.id}>
                <div className="job-section-heading">
                  <button
                    aria-controls={`job-section-${section.id}`}
                    aria-expanded={sectionIsExpanded(section.id)}
                    className="job-section-toggle"
                    disabled={
                      section.id === "active" &&
                      section.jobs.some((job) => job.status === "running")
                    }
                    onClick={() => toggleSection(section.id)}
                    type="button"
                  >
                    {sectionIsExpanded(section.id) ? (
                      <ChevronDown size={15} />
                    ) : (
                      <ChevronRight size={15} />
                    )}
                    <h3>
                      {t(`jobs.sections.${section.id}.title`)} (
                      {section.jobs.length})
                    </h3>
                  </button>
                  {section.id !== "active" && (
                    <button
                      aria-label={t("jobs.sections.clearLabel", {
                        section: t(`jobs.sections.${section.id}.title`),
                      })}
                      onClick={() => void clearSection(section.id)}
                      type="button"
                    >
                      <Trash2 size={14} />
                      {t("jobs.sections.clear")}
                    </button>
                  )}
                </div>
                {sectionIsExpanded(section.id) && (
                  <div className="job-list" id={`job-section-${section.id}`}>
                    {section.jobs.map(renderJob)}
                  </div>
                )}
              </section>
            ),
        )}
      </div>
    </div>
  );
}

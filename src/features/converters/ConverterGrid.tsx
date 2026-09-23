import {
  FileAudio,
  FileImage,
  FileOutput,
  FileText,
  Files,
  Wrench,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import type { ConverterCapability } from "../../types/bootstrap";

const icons = {
  "images-to-pdf": FileImage,
  "pdf-tools": Wrench,
  "pdf-to-images": Files,
  audio: FileAudio,
  "office-to-pdf": FileOutput,
  "pdf-to-docx": FileText,
};

interface ConverterGridProps {
  converters: ConverterCapability[];
  loading: boolean;
  onSelect?: (id: string) => void;
}

export function ConverterGrid({
  converters,
  loading,
  onSelect,
}: ConverterGridProps) {
  const { t } = useTranslation();

  if (loading) {
    return (
      <div className="converter-grid" aria-label={t("common.loading")}>
        {Array.from({ length: 6 }, (_, index) => (
          <div className="skeleton" key={index} aria-hidden="true" />
        ))}
      </div>
    );
  }

  return (
    <div className="converter-grid">
      {converters.map((converter) => {
        const Icon = icons[converter.id as keyof typeof icons] ?? FileOutput;
        return (
          <button
            className="converter-card"
            disabled={converter.status === "planned"}
            key={converter.id}
            onClick={() => onSelect?.(converter.id)}
            type="button"
          >
            <div className="converter-card-header">
              <span className="converter-icon">
                <Icon size={18} aria-hidden="true" />
              </span>
              <span className="status-pill" data-status={converter.status}>
                {t(`status.${converter.status}`)}
              </span>
            </div>
            <h3>{t(converter.titleKey)}</h3>
            <p>{t(converter.descriptionKey)}</p>
            <span className="engine-label">
              {converter.engine} ·{" "}
              {converter.bundled ? t("status.bundled") : t("status.external")}
            </span>
          </button>
        );
      })}
    </div>
  );
}

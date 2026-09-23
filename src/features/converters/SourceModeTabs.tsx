import { File, Files } from "lucide-react";
import { useTranslation } from "react-i18next";

export type SourceMode = "file" | "folder";

interface SourceModeTabsProps {
  busy: boolean;
  mode: SourceMode;
  onChange: (mode: SourceMode) => void;
}

export function SourceModeTabs({ busy, mode, onChange }: SourceModeTabsProps) {
  const { t } = useTranslation();
  return (
    <div
      aria-label={t("batchMode.label")}
      className="audio-mode-tabs"
      role="tablist"
    >
      <button
        aria-selected={mode === "file"}
        data-active={mode === "file"}
        disabled={busy}
        onClick={() => onChange("file")}
        role="tab"
        type="button"
      >
        <File size={15} />
        {t("batchMode.file")}
      </button>
      <button
        aria-selected={mode === "folder"}
        data-active={mode === "folder"}
        disabled={busy}
        onClick={() => onChange("folder")}
        role="tab"
        type="button"
      >
        <Files size={15} />
        {t("batchMode.folder")}
      </button>
    </div>
  );
}

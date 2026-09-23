import {
  AlertTriangle,
  CheckCircle2,
  FolderOpen,
  RefreshCw,
  RotateCcw,
} from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type {
  DependencyId,
  DependencyStatus,
  DependencyStatuses,
} from "../../types/bootstrap";

interface SettingsProps {
  dependencies: DependencyStatuses | null;
  desktopAvailable: boolean;
  onBrowse: (dependency: DependencyId) => Promise<void>;
  onReset: (dependency: DependencyId) => Promise<void>;
  onRescan: () => Promise<void>;
}

function DependencyEntry({
  dependency,
  name,
  status,
  desktopAvailable,
  onBrowse,
  onReset,
  onRescan,
  rescanBusy,
}: {
  dependency: DependencyId;
  name: string;
  status: DependencyStatus | null;
  desktopAvailable: boolean;
  onBrowse: (dependency: DependencyId) => Promise<void>;
  onReset: (dependency: DependencyId) => Promise<void>;
  onRescan: (dependency: DependencyId) => Promise<void>;
  rescanBusy: boolean;
}) {
  const { t } = useTranslation();
  const invalidOverride =
    status?.issue === "invalid_manual_path" ||
    status?.issue === "windows_apps_alias";
  const path = status?.manualPath ?? status?.automaticPath;

  return (
    <article className="dependency-setting">
      <div className="dependency-setting-main">
        <span
          className="dependency-status-icon"
          data-ready={status?.ready ?? false}
        >
          {status?.ready ? (
            <CheckCircle2 size={17} />
          ) : (
            <AlertTriangle size={17} />
          )}
        </span>
        <div>
          <div className="dependency-setting-title">
            <strong>{name}</strong>
            <span>
              {t(
                status?.ready
                  ? "settings.dependencies.ready"
                  : "settings.dependencies.notReady",
              )}
            </span>
          </div>
          {status?.manualPath ? (
            <>
              <p>{t("settings.dependencies.manualPath")}</p>
              <code>{status.manualPath}</code>
            </>
          ) : status?.automaticPath ? (
            <>
              <p>{t("settings.dependencies.autoPath")}</p>
              <code>{status.automaticPath}</code>
            </>
          ) : (
            <p>{t("settings.dependencies.notFound")}</p>
          )}
          {invalidOverride && (
            <p className="dependency-warning" role="alert">
              {status?.issue === "windows_apps_alias"
                ? t("settings.dependencies.windowsAppsAlias")
                : t("settings.dependencies.invalidPath", {
                    fallback:
                      status?.automaticPath ??
                      t("settings.dependencies.noFallback"),
                  })}
            </p>
          )}
          {status?.issue === "version_check_failed" && (
            <p className="dependency-warning" role="alert">
              {t("settings.dependencies.versionCheckFailed")}
            </p>
          )}
          {status?.issue === "packages_missing" && (
            <p className="dependency-warning" role="status">
              {t("settings.dependencies.packagesMissing")}
            </p>
          )}
          {!invalidOverride &&
            path &&
            !status?.ready &&
            status?.issue !== "packages_missing" &&
            status?.issue !== "version_check_failed" && (
              <p className="dependency-warning">
                {t("settings.dependencies.unavailable")}
              </p>
            )}
        </div>
      </div>
      <div className="dependency-setting-actions">
        <button
          disabled={!desktopAvailable}
          onClick={() => void onBrowse(dependency)}
          type="button"
        >
          <FolderOpen size={15} />
          {t("settings.dependencies.browse")}
        </button>
        {status?.manualPath && (
          <button onClick={() => void onReset(dependency)} type="button">
            <RotateCcw size={15} />
            {t("settings.dependencies.reset")}
          </button>
        )}
        {!status?.manualPath && (
          <button
            disabled={!desktopAvailable || rescanBusy}
            onClick={() => void onRescan(dependency)}
            type="button"
          >
            <RefreshCw className={rescanBusy ? "spin" : undefined} size={15} />
            {t(
              rescanBusy
                ? "settings.dependencies.searching"
                : "settings.dependencies.searchAgain",
            )}
          </button>
        )}
      </div>
    </article>
  );
}

export function Settings({
  dependencies,
  desktopAvailable,
  onBrowse,
  onReset,
  onRescan,
}: SettingsProps) {
  const { t } = useTranslation();
  const [rescanning, setRescanning] = useState<DependencyId | null>(null);

  async function rescan(dependency: DependencyId) {
    setRescanning(dependency);
    try {
      await onRescan();
    } finally {
      setRescanning(null);
    }
  }

  return (
    <div className="settings-workspace">
      <div className="section-heading">
        <div>
          <p className="eyebrow">{t("settings.eyebrow")}</p>
          <h2>{t("settings.title")}</h2>
          <p>{t("settings.description")}</p>
        </div>
      </div>
      <section
        className="settings-panel"
        aria-labelledby="dependencies-heading"
      >
        <div className="settings-panel-heading">
          <h3 id="dependencies-heading">{t("settings.dependencies.title")}</h3>
          <p>{t("settings.dependencies.description")}</p>
        </div>
        <DependencyEntry
          dependency="libreoffice"
          desktopAvailable={desktopAvailable}
          name="LibreOffice"
          onBrowse={onBrowse}
          onReset={onReset}
          onRescan={rescan}
          rescanBusy={rescanning === "libreoffice"}
          status={dependencies?.libreoffice ?? null}
        />
        <DependencyEntry
          dependency="python"
          desktopAvailable={desktopAvailable}
          name="Python"
          onBrowse={onBrowse}
          onReset={onReset}
          onRescan={rescan}
          rescanBusy={rescanning === "python"}
          status={dependencies?.python ?? null}
        />
      </section>
    </div>
  );
}

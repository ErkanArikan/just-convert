import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  FileStack,
  Files,
  FolderOpen,
  ListChecks,
  LockKeyhole,
  Settings2,
} from "lucide-react";
import { ConverterGrid } from "./features/converters/ConverterGrid";
import { AudioConverter } from "./features/converters/AudioConverter";
import { ImagesToPdf } from "./features/converters/ImagesToPdf";
import { PdfTools } from "./features/converters/PdfTools";
import { PdfToImages } from "./features/converters/PdfToImages";
import { OfficeToPdf } from "./features/converters/OfficeToPdf";
import { PdfToDocx } from "./features/converters/PdfToDocx";
import { FileBrowser } from "./features/files/FileBrowser";
import { JobList } from "./features/jobs/JobList";
import { Settings } from "./features/settings/Settings";
import { isTauri } from "@tauri-apps/api/core";
import {
  chooseDependencyExecutable,
  getAppBootstrap,
  setDependencyOverride,
} from "./services/backend";
import { usePreferencesStore } from "./store/preferences";
import type { AppBootstrap, DependencyId } from "./types/bootstrap";

const navigation = [
  { id: "files", label: "navigation.files", icon: Files },
  { id: "convert", label: "navigation.convert", icon: FileStack },
  { id: "jobs", label: "navigation.jobs", icon: ListChecks },
  { id: "settings", label: "navigation.settings", icon: Settings2 },
] as const;

type SectionId = (typeof navigation)[number]["id"];

function useThemePreference() {
  const theme = usePreferencesStore((state) => state.theme);

  useEffect(() => {
    const media = window.matchMedia?.("(prefers-color-scheme: dark)");
    const applyTheme = () => {
      const resolved =
        theme === "system" ? (media?.matches ? "dark" : "light") : theme;
      document.documentElement.dataset.theme = resolved;
      document.documentElement.style.colorScheme = resolved;
    };

    applyTheme();
    media?.addEventListener("change", applyTheme);
    return () => media?.removeEventListener("change", applyTheme);
  }, [theme]);
}

function App() {
  const { t, i18n } = useTranslation();
  const [activeSection, setActiveSection] = useState<SectionId>("convert");
  const [bootstrap, setBootstrap] = useState<AppBootstrap | null>(null);
  const [loadError, setLoadError] = useState(false);
  const [activeTool, setActiveTool] = useState<string | null>(null);
  const [dependencyActionError, setDependencyActionError] = useState<
    string | null
  >(null);
  const language = usePreferencesStore((state) => state.language);
  const setLanguage = usePreferencesStore((state) => state.setLanguage);
  const theme = usePreferencesStore((state) => state.theme);
  const setTheme = usePreferencesStore((state) => state.setTheme);

  useThemePreference();

  useEffect(() => {
    void i18n.changeLanguage(language);
  }, [i18n, language]);

  useEffect(() => {
    let mounted = true;
    getAppBootstrap()
      .then((result) => {
        if (mounted) setBootstrap(result);
      })
      .catch(() => {
        if (mounted) setLoadError(true);
      });
    return () => {
      mounted = false;
    };
  }, []);

  async function refreshBootstrap() {
    const result = await getAppBootstrap();
    setBootstrap(result);
    setLoadError(false);
  }

  async function browseDependency(dependency: DependencyId) {
    setDependencyActionError(null);
    try {
      const path = await chooseDependencyExecutable(dependency);
      if (!path) return;
      await setDependencyOverride(dependency, path);
      await refreshBootstrap();
    } catch (reason) {
      setDependencyActionError(String(reason));
    }
  }

  async function resetDependency(dependency: DependencyId) {
    setDependencyActionError(null);
    try {
      await setDependencyOverride(dependency, null);
      await refreshBootstrap();
    } catch (reason) {
      setDependencyActionError(String(reason));
    }
  }

  async function rescanDependencies() {
    setDependencyActionError(null);
    try {
      await refreshBootstrap();
    } catch (reason) {
      setDependencyActionError(String(reason));
    }
  }

  const pageTitle = useMemo(
    () =>
      t(
        navigation.find((item) => item.id === activeSection)?.label ??
          "navigation.convert",
      ),
    [activeSection, t],
  );

  return (
    <div className="app-frame">
      <aside className="sidebar">
        <div className="brand" aria-label={t("app.name")}>
          <span className="brand-mark" aria-hidden="true">
            <img alt="" src="/app-icon.png" />
          </span>
          <span>
            <strong>{t("app.name")}</strong>
            <small>{t("app.localWorkspace")}</small>
          </span>
        </div>

        <nav className="primary-nav" aria-label={t("navigation.primary")}>
          {navigation.map(({ id, label, icon: Icon }) => (
            <button
              className="nav-item"
              data-active={activeSection === id}
              key={id}
              onClick={() => setActiveSection(id)}
              type="button"
            >
              <Icon size={18} aria-hidden="true" />
              <span>{t(label)}</span>
            </button>
          ))}
        </nav>

        <div className="sidebar-footer">
          <div className="local-badge">
            <LockKeyhole size={14} aria-hidden="true" />
            <span>{t("app.localOnly")}</span>
          </div>
        </div>
      </aside>

      <main className="workspace">
        <header className="topbar">
          <div>
            <p className="eyebrow">{t("app.localWorkspace")}</p>
            <h1>{pageTitle}</h1>
          </div>
          <div className="preferences" aria-label={t("preferences.label")}>
            <label>
              <span className="sr-only">{t("preferences.language")}</span>
              <select
                aria-label={t("preferences.language")}
                value={language}
                onChange={(event) =>
                  setLanguage(event.target.value as "en" | "tr")
                }
              >
                <option value="en">English</option>
                <option value="tr">Türkçe</option>
              </select>
            </label>
            <label>
              <span className="sr-only">{t("preferences.theme")}</span>
              <select
                aria-label={t("preferences.theme")}
                value={theme}
                onChange={(event) =>
                  setTheme(event.target.value as "system" | "light" | "dark")
                }
              >
                <option value="system">{t("preferences.system")}</option>
                <option value="light">{t("preferences.light")}</option>
                <option value="dark">{t("preferences.dark")}</option>
              </select>
            </label>
          </div>
        </header>

        <section className="content">
          {dependencyActionError && (
            <div className="inline-error" role="alert">
              {dependencyActionError}
            </div>
          )}
          {activeSection === "convert" && activeTool === "audio" ? (
            <AudioConverter
              desktopAvailable={isTauri()}
              onBack={() => setActiveTool(null)}
              onViewJobs={() => setActiveSection("jobs")}
            />
          ) : activeSection === "convert" && activeTool === "images-to-pdf" ? (
            <ImagesToPdf
              desktopAvailable={isTauri()}
              onBack={() => setActiveTool(null)}
              onViewJobs={() => setActiveSection("jobs")}
            />
          ) : activeSection === "convert" && activeTool === "pdf-tools" ? (
            <PdfTools
              desktopAvailable={isTauri()}
              onBack={() => setActiveTool(null)}
              onViewJobs={() => setActiveSection("jobs")}
            />
          ) : activeSection === "convert" && activeTool === "pdf-to-images" ? (
            <PdfToImages
              desktopAvailable={isTauri()}
              onBack={() => setActiveTool(null)}
              onViewJobs={() => setActiveSection("jobs")}
            />
          ) : activeSection === "convert" && activeTool === "office-to-pdf" ? (
            <OfficeToPdf
              desktopAvailable={isTauri()}
              engineAvailable={
                bootstrap?.converters.find(
                  (converter) => converter.id === "office-to-pdf",
                )?.status === "ready"
              }
              onBack={() => setActiveTool(null)}
              onSetManualPath={() => void browseDependency("libreoffice")}
              onViewJobs={() => setActiveSection("jobs")}
            />
          ) : activeSection === "convert" && activeTool === "pdf-to-docx" ? (
            <PdfToDocx
              desktopAvailable={isTauri()}
              engineAvailable={
                bootstrap?.converters.find(
                  (converter) => converter.id === "pdf-to-docx",
                )?.status === "ready"
              }
              onBack={() => setActiveTool(null)}
              onSetManualPath={() => void browseDependency("python")}
              onViewJobs={() => setActiveSection("jobs")}
            />
          ) : activeSection === "convert" ? (
            <>
              <div className="hero">
                <div>
                  <p className="eyebrow accent">
                    {t("dashboard.privateEyebrow")}
                  </p>
                  <h2>{t("dashboard.title")}</h2>
                  <p>{t("dashboard.description")}</p>
                </div>
                <button
                  className="primary-button"
                  onClick={() => setActiveSection("files")}
                  type="button"
                >
                  <FolderOpen size={18} aria-hidden="true" />
                  {t("dashboard.browse")}
                </button>
              </div>

              <div className="section-heading">
                <div>
                  <p className="eyebrow">{t("converters.eyebrow")}</p>
                  <h2>{t("converters.title")}</h2>
                </div>
                {bootstrap && (
                  <span className="version-pill">
                    {bootstrap.platform} · v{bootstrap.appVersion}
                  </span>
                )}
              </div>

              {loadError ? (
                <div className="notice" role="alert">
                  {t("errors.bootstrap")}
                </div>
              ) : (
                <ConverterGrid
                  converters={bootstrap?.converters ?? []}
                  loading={!bootstrap}
                  onSelect={setActiveTool}
                />
              )}
            </>
          ) : activeSection === "files" ? (
            <FileBrowser desktopAvailable={isTauri()} />
          ) : activeSection === "jobs" ? (
            <JobList desktopAvailable={isTauri()} />
          ) : (
            <Settings
              dependencies={bootstrap?.dependencies ?? null}
              desktopAvailable={isTauri()}
              onBrowse={browseDependency}
              onRescan={rescanDependencies}
              onReset={resetDependency}
            />
          )}
        </section>
      </main>
    </div>
  );
}

export default App;

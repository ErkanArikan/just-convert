import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  ArrowUp,
  Copy,
  File,
  Folder,
  FolderOpen,
  FolderPlus,
  Move,
  Pencil,
  RefreshCw,
  Search,
  Trash2,
  X,
} from "lucide-react";
import {
  chooseDirectory,
  copyEntries,
  createDirectory,
  deleteEntriesPermanently,
  listDirectory,
  moveEntries,
  renameEntry,
  trashEntries,
} from "../../services/backend";
import type { DirectoryListing, FileEntry } from "../../types/fileManager";

function formatSize(bytes: number | null) {
  if (bytes === null) return "—";
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(value >= 10 ? 0 : 1)} ${units[unit]}`;
}

interface FileBrowserProps {
  desktopAvailable: boolean;
}

export function FileBrowser({ desktopAvailable }: FileBrowserProps) {
  const { t, i18n } = useTranslation();
  const [listing, setListing] = useState<DirectoryListing | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [query, setQuery] = useState("");
  const [showHidden, setShowHidden] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const visibleEntries = useMemo(() => {
    const normalizedQuery = query.trim().toLocaleLowerCase(i18n.language);
    return (listing?.entries ?? []).filter(
      (entry) =>
        (showHidden || !entry.hidden) &&
        (!normalizedQuery ||
          entry.name
            .toLocaleLowerCase(i18n.language)
            .includes(normalizedQuery)),
    );
  }, [i18n.language, listing?.entries, query, showHidden]);

  async function load(path: string) {
    setBusy(true);
    setError(null);
    try {
      setListing(await listDirectory(path));
      setSelected(new Set());
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }

  async function pickFolder() {
    try {
      const path = await chooseDirectory();
      if (path) await load(path);
    } catch (reason) {
      setError(String(reason));
    }
  }

  function toggleSelection(path: string) {
    setSelected((current) => {
      const next = new Set(current);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  }

  async function runOperation(operation: () => Promise<unknown>) {
    if (!listing) return;
    setBusy(true);
    setError(null);
    try {
      await operation();
      await load(listing.path);
    } catch (reason) {
      setError(String(reason));
      setBusy(false);
    }
  }

  async function makeFolder() {
    if (!listing) return;
    const name = window.prompt(t("files.prompts.folderName"));
    if (name) await runOperation(() => createDirectory(listing.path, name));
  }

  async function renameSelected() {
    const path = [...selected][0];
    const entry = listing?.entries.find((item) => item.path === path);
    if (!entry) return;
    const name = window.prompt(t("files.prompts.newName"), entry.name);
    if (name && name !== entry.name)
      await runOperation(() => renameEntry(path, name));
  }

  async function transfer(mode: "copy" | "move") {
    const destination = await chooseDirectory();
    if (!destination) return;
    const sources = [...selected];
    await runOperation(() =>
      mode === "copy"
        ? copyEntries(sources, destination)
        : moveEntries(sources, destination),
    );
  }

  async function moveToTrash() {
    if (
      !window.confirm(t("files.confirmations.trash", { count: selected.size }))
    )
      return;
    await runOperation(() => trashEntries([...selected]));
  }

  async function permanentlyDelete() {
    if (
      !window.confirm(
        t("files.confirmations.permanent", { count: selected.size }),
      )
    )
      return;
    await runOperation(() => deleteEntriesPermanently([...selected]));
  }

  function openEntry(entry: FileEntry) {
    if (entry.kind === "directory") void load(entry.path);
  }

  if (!listing) {
    return (
      <div className="file-empty-state">
        <span className="empty-icon">
          <FolderOpen size={24} aria-hidden="true" />
        </span>
        <h2>{t("files.empty.title")}</h2>
        <p>
          {desktopAvailable
            ? t("files.empty.description")
            : t("files.empty.browserDescription")}
        </p>
        <button
          className="primary-button"
          disabled={!desktopAvailable}
          onClick={() => void pickFolder()}
          type="button"
        >
          <FolderOpen size={18} aria-hidden="true" />
          {t("files.actions.chooseFolder")}
        </button>
        {error && <div className="inline-error">{error}</div>}
      </div>
    );
  }

  return (
    <div className="file-browser">
      <div className="file-toolbar">
        <div className="toolbar-group">
          <button
            disabled={!listing.parent || busy}
            onClick={() => listing.parent && void load(listing.parent)}
            title={t("files.actions.up")}
            type="button"
          >
            <ArrowUp size={17} />
          </button>
          <button
            disabled={busy}
            onClick={() => void load(listing.path)}
            title={t("files.actions.refresh")}
            type="button"
          >
            <RefreshCw size={17} />
          </button>
          <button
            disabled={busy}
            onClick={() => void pickFolder()}
            title={t("files.actions.chooseFolder")}
            type="button"
          >
            <FolderOpen size={17} />
          </button>
        </div>
        <div className="path-chip" title={listing.path}>
          {listing.path}
        </div>
        <label className="search-box">
          <Search size={15} aria-hidden="true" />
          <input
            aria-label={t("files.search")}
            onChange={(event) => setQuery(event.target.value)}
            placeholder={t("files.search")}
            value={query}
          />
          {query && (
            <button
              onClick={() => setQuery("")}
              title={t("files.actions.clearSearch")}
              type="button"
            >
              <X size={14} />
            </button>
          )}
        </label>
      </div>

      <div className="operation-bar">
        <button disabled={busy} onClick={() => void makeFolder()} type="button">
          <FolderPlus size={15} />
          {t("files.actions.newFolder")}
        </button>
        <button
          disabled={selected.size !== 1 || busy}
          onClick={() => void renameSelected()}
          type="button"
        >
          <Pencil size={15} />
          {t("files.actions.rename")}
        </button>
        <button
          disabled={!selected.size || busy}
          onClick={() => void transfer("copy")}
          type="button"
        >
          <Copy size={15} />
          {t("files.actions.copy")}
        </button>
        <button
          disabled={!selected.size || busy}
          onClick={() => void transfer("move")}
          type="button"
        >
          <Move size={15} />
          {t("files.actions.move")}
        </button>
        <button
          disabled={!selected.size || busy}
          onClick={() => void moveToTrash()}
          type="button"
        >
          <Trash2 size={15} />
          {t("files.actions.trash")}
        </button>
        <button
          className="danger-action"
          disabled={!selected.size || busy}
          onClick={() => void permanentlyDelete()}
          type="button"
        >
          {t("files.actions.deletePermanently")}
        </button>
        <label className="hidden-toggle">
          <input
            checked={showHidden}
            onChange={(event) => setShowHidden(event.target.checked)}
            type="checkbox"
          />
          {t("files.showHidden")}
        </label>
      </div>

      {error && (
        <div className="inline-error" role="alert">
          {error}
        </div>
      )}
      <div className="file-table-wrap" aria-busy={busy}>
        <table className="file-table">
          <thead>
            <tr>
              <th className="check-cell">
                <span className="sr-only">{t("files.columns.select")}</span>
              </th>
              <th>{t("files.columns.name")}</th>
              <th>{t("files.columns.type")}</th>
              <th>{t("files.columns.modified")}</th>
              <th>{t("files.columns.size")}</th>
            </tr>
          </thead>
          <tbody>
            {visibleEntries.map((entry) => (
              <tr
                data-selected={selected.has(entry.path)}
                key={entry.path}
                onDoubleClick={() => openEntry(entry)}
              >
                <td className="check-cell">
                  <input
                    aria-label={t("files.selectEntry", { name: entry.name })}
                    checked={selected.has(entry.path)}
                    onChange={() => toggleSelection(entry.path)}
                    type="checkbox"
                  />
                </td>
                <td>
                  <button
                    className="entry-name"
                    onClick={() =>
                      entry.kind === "directory"
                        ? openEntry(entry)
                        : toggleSelection(entry.path)
                    }
                    type="button"
                  >
                    {entry.kind === "directory" ? (
                      <Folder size={17} />
                    ) : (
                      <File size={17} />
                    )}
                    <span>{entry.name}</span>
                  </button>
                </td>
                <td>
                  {entry.kind === "directory"
                    ? t("files.types.folder")
                    : (entry.extension?.toUpperCase() ?? t("files.types.file"))}
                </td>
                <td>
                  {entry.modifiedAt
                    ? new Intl.DateTimeFormat(i18n.language, {
                        dateStyle: "medium",
                        timeStyle: "short",
                      }).format(new Date(entry.modifiedAt * 1000))
                    : "—"}
                </td>
                <td>{formatSize(entry.sizeBytes)}</td>
              </tr>
            ))}
          </tbody>
        </table>
        {!visibleEntries.length && (
          <div className="table-empty">{t("files.emptyFolder")}</div>
        )}
      </div>
    </div>
  );
}

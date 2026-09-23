use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::platform::{self, PdfToDocxRuntime};

const LIBREOFFICE_KEY: &str = "libreoffice";
const PYTHON_KEY: &str = "python";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyStatus {
    pub id: String,
    pub automatic_path: Option<String>,
    pub manual_path: Option<String>,
    pub manual_path_valid: bool,
    pub resolved_path: Option<String>,
    pub ready: bool,
    pub issue: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyStatuses {
    pub libreoffice: DependencyStatus,
    pub python: DependencyStatus,
}

#[derive(Default)]
struct Overrides {
    libreoffice: Option<PathBuf>,
    python: Option<PathBuf>,
}

pub struct DependencyService {
    database_path: PathBuf,
    resource_directory: Option<PathBuf>,
    overrides: Mutex<Overrides>,
}

impl DependencyService {
    pub fn new(data_directory: &Path, resource_directory: Option<PathBuf>) -> Result<Self, String> {
        fs::create_dir_all(data_directory)
            .map_err(|error| format!("Application data directory could not be created: {error}"))?;
        let database_path = data_directory.join("settings.sqlite3");
        let connection = open_database(&database_path)?;
        let overrides = Overrides {
            libreoffice: read_override(&connection, LIBREOFFICE_KEY)?,
            python: read_override(&connection, PYTHON_KEY)?,
        };
        Ok(Self {
            database_path,
            resource_directory,
            overrides: Mutex::new(overrides),
        })
    }

    pub fn statuses(&self) -> DependencyStatuses {
        let automatic_libreoffice = platform::find_libreoffice();
        let automatic_python = platform::find_python();
        let worker_available =
            platform::find_pdf_to_docx_worker(self.resource_directory.clone()).is_some();
        let overrides = self
            .overrides
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        let manual_libreoffice_valid = overrides
            .libreoffice
            .as_deref()
            .is_some_and(is_libreoffice_executable);
        let resolved_libreoffice = overrides
            .libreoffice
            .as_ref()
            .filter(|_| manual_libreoffice_valid)
            .cloned()
            .or_else(|| automatic_libreoffice.clone());

        let manual_python_eligible = overrides
            .python
            .as_deref()
            .is_some_and(|path| path.is_file() && !platform::is_windows_apps_python_alias(path));
        let manual_python_version_valid = overrides
            .python
            .as_deref()
            .filter(|path| path.is_file() && !platform::is_windows_apps_python_alias(path))
            .is_some_and(platform::python_is_usable);
        let resolved_python = overrides
            .python
            .as_ref()
            .filter(|_| manual_python_eligible)
            .cloned()
            .or_else(|| automatic_python.clone());
        let python_packages_ready = resolved_python
            .as_deref()
            .is_some_and(platform::python_has_pdf_to_docx_packages);

        DependencyStatuses {
            libreoffice: DependencyStatus {
                id: LIBREOFFICE_KEY.into(),
                automatic_path: path_string(automatic_libreoffice.as_deref()),
                manual_path: path_string(overrides.libreoffice.as_deref()),
                manual_path_valid: manual_libreoffice_valid,
                resolved_path: path_string(resolved_libreoffice.as_deref()),
                ready: resolved_libreoffice.is_some(),
                issue: invalid_override_issue(
                    overrides.libreoffice.as_ref(),
                    manual_libreoffice_valid,
                ),
            },
            python: DependencyStatus {
                id: PYTHON_KEY.into(),
                automatic_path: path_string(automatic_python.as_deref()),
                manual_path: path_string(overrides.python.as_deref()),
                manual_path_valid: manual_python_eligible,
                resolved_path: path_string(resolved_python.as_deref()),
                ready: resolved_python.is_some() && python_packages_ready && worker_available,
                issue: if overrides
                    .python
                    .as_deref()
                    .is_some_and(platform::is_windows_apps_python_alias)
                {
                    Some("windows_apps_alias".into())
                } else if overrides.python.is_some() && !manual_python_eligible {
                    Some("invalid_manual_path".into())
                } else if overrides.python.is_some() && !manual_python_version_valid {
                    Some("version_check_failed".into())
                } else if resolved_python.is_some() && !python_packages_ready {
                    Some("packages_missing".into())
                } else if resolved_python.is_some() && !worker_available {
                    Some("worker_missing".into())
                } else {
                    None
                },
            },
        }
    }

    pub fn libreoffice_path(&self) -> Option<PathBuf> {
        let manual = self
            .overrides
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .libreoffice
            .clone();
        manual
            .filter(|path| is_libreoffice_executable(path))
            .or_else(platform::find_libreoffice)
    }

    pub fn pdf_to_docx_runtime(&self) -> Option<PdfToDocxRuntime> {
        let manual = self
            .overrides
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .python
            .clone();
        let python_path = manual
            .filter(|path| path.is_file() && !platform::is_windows_apps_python_alias(path))
            .or_else(platform::find_python)?;
        let worker_path = platform::find_pdf_to_docx_worker(self.resource_directory.clone())?;
        platform::python_has_pdf_to_docx_packages(&python_path).then_some(PdfToDocxRuntime {
            python_path,
            worker_path,
        })
    }

    pub fn set_override(&self, dependency: &str, path: Option<PathBuf>) -> Result<(), String> {
        let key = match dependency {
            LIBREOFFICE_KEY => LIBREOFFICE_KEY,
            PYTHON_KEY => PYTHON_KEY,
            _ => return Err("Unknown optional dependency".into()),
        };
        if let Some(path) = &path {
            if !path.is_absolute() || !path.is_file() {
                return Err("Select an existing executable using an absolute path".into());
            }
            if key == LIBREOFFICE_KEY && !is_libreoffice_executable(path) {
                return Err("The selected file is not soffice.exe".into());
            }
            if key == PYTHON_KEY && platform::is_windows_apps_python_alias(path) {
                return Err(
                    "The Microsoft Store Python alias is not a real installation. Select a Python executable outside WindowsApps."
                        .into(),
                );
            }
        }

        let connection = open_database(&self.database_path)?;
        if let Some(path) = &path {
            connection
                .execute(
                    "INSERT INTO dependency_settings (dependency, executable_path) VALUES (?1, ?2) \
                     ON CONFLICT(dependency) DO UPDATE SET executable_path = excluded.executable_path",
                    params![key, path.to_string_lossy()],
                )
                .map_err(|error| format!("Dependency setting could not be saved: {error}"))?;
        } else {
            connection
                .execute(
                    "DELETE FROM dependency_settings WHERE dependency = ?1",
                    params![key],
                )
                .map_err(|error| format!("Dependency setting could not be reset: {error}"))?;
        }

        let mut overrides = self
            .overrides
            .lock()
            .map_err(|_| "Dependency settings lock failed")?;
        match key {
            LIBREOFFICE_KEY => overrides.libreoffice = path,
            PYTHON_KEY => overrides.python = path,
            _ => unreachable!(),
        }
        Ok(())
    }
}

fn open_database(path: &Path) -> Result<Connection, String> {
    let connection = Connection::open(path)
        .map_err(|error| format!("Dependency settings database could not be opened: {error}"))?;
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS dependency_settings (
                dependency TEXT PRIMARY KEY NOT NULL,
                executable_path TEXT NOT NULL
            );",
        )
        .map_err(|error| {
            format!("Dependency settings database could not be initialized: {error}")
        })?;
    Ok(connection)
}

fn read_override(connection: &Connection, dependency: &str) -> Result<Option<PathBuf>, String> {
    connection
        .query_row(
            "SELECT executable_path FROM dependency_settings WHERE dependency = ?1",
            params![dependency],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map(|value| value.map(PathBuf::from))
        .map_err(|error| format!("Dependency setting could not be read: {error}"))
}

fn is_libreoffice_executable(path: &Path) -> bool {
    path.is_file()
        && path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| {
                value.eq_ignore_ascii_case("soffice.exe")
                    || value.eq_ignore_ascii_case("soffice.com")
                    || value.eq_ignore_ascii_case("soffice")
                    || value.eq_ignore_ascii_case("libreoffice")
            })
}

fn path_string(path: Option<&Path>) -> Option<String> {
    path.map(|value| value.to_string_lossy().into_owned())
}

fn invalid_override_issue(path: Option<&PathBuf>, valid: bool) -> Option<String> {
    (path.is_some() && !valid).then(|| "invalid_manual_path".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn manual_override_round_trips_through_sqlite() {
        let directory = tempdir().unwrap();
        let executable = directory.path().join(if cfg!(windows) {
            "soffice.exe"
        } else {
            "soffice"
        });
        fs::write(&executable, b"placeholder").unwrap();
        let service = DependencyService::new(directory.path(), None).unwrap();
        service
            .set_override(LIBREOFFICE_KEY, Some(executable.clone()))
            .unwrap();

        let recreated = DependencyService::new(directory.path(), None).unwrap();
        assert_eq!(
            recreated.statuses().libreoffice.manual_path,
            Some(executable.to_string_lossy().into_owned())
        );
        recreated.set_override(LIBREOFFICE_KEY, None).unwrap();
        assert!(recreated.statuses().libreoffice.manual_path.is_none());
    }

    #[test]
    fn python_override_is_saved_when_the_version_probe_fails() {
        let directory = tempdir().unwrap();
        let executable = std::env::current_exe().unwrap();
        let service = DependencyService::new(directory.path(), None).unwrap();

        service
            .set_override(PYTHON_KEY, Some(executable.clone()))
            .unwrap();
        let status = service.statuses().python;

        assert_eq!(
            status.manual_path,
            Some(executable.to_string_lossy().into_owned())
        );
        assert!(status.manual_path_valid);
        assert_eq!(status.issue.as_deref(), Some("version_check_failed"));
        assert_eq!(
            status.resolved_path,
            Some(executable.to_string_lossy().into_owned())
        );
    }
}

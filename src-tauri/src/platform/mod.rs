use std::{
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[cfg(target_os = "windows")]
use std::fs;

#[cfg(target_os = "windows")]
use winreg::{
    enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE},
    RegKey,
};

#[derive(Clone, Debug)]
pub struct PdfToDocxRuntime {
    pub python_path: PathBuf,
    pub worker_path: PathBuf,
}

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Creates a child process without allocating a visible console window on Windows.
pub(crate) fn background_command<S: AsRef<OsStr>>(program: S) -> Command {
    let mut command = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;

        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

fn find_on_path(file_names: &[&str]) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .flat_map(|directory| file_names.iter().map(move |name| directory.join(name)))
        .find(|candidate| candidate.is_file())
}

pub fn find_libreoffice() -> Option<PathBuf> {
    if let Some(path) = find_on_path(if cfg!(windows) {
        &["soffice.com", "soffice.exe"]
    } else {
        &["soffice", "libreoffice"]
    }) {
        return Some(path);
    }

    #[cfg(target_os = "windows")]
    if let Some(path) = find_libreoffice_in_registry() {
        return Some(path);
    }

    #[cfg(target_os = "windows")]
    let candidates = [
        PathBuf::from(r"C:\Program Files\LibreOffice\program\soffice.exe"),
        PathBuf::from(r"C:\Program Files (x86)\LibreOffice\program\soffice.exe"),
    ];

    #[cfg(target_os = "macos")]
    let candidates = [PathBuf::from(
        "/Applications/LibreOffice.app/Contents/MacOS/soffice",
    )];

    #[cfg(target_os = "linux")]
    let candidates: [PathBuf; 0] = [];

    candidates.into_iter().find(|candidate| candidate.is_file())
}

#[cfg(target_os = "windows")]
fn executable_from_install_path(value: &str, executable: &str) -> Option<PathBuf> {
    let path = PathBuf::from(value.trim_matches(['"', ' ']));
    let candidates = if path.is_file() {
        vec![path]
    } else {
        vec![path.join(executable), path.join("program").join(executable)]
    };
    candidates.into_iter().find(|candidate| candidate.is_file())
}

#[cfg(target_os = "windows")]
fn find_libreoffice_in_registry() -> Option<PathBuf> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    for key_name in [
        r"SOFTWARE\LibreOffice",
        r"SOFTWARE\LibreOffice\UNO\InstallPath",
        r"SOFTWARE\WOW6432Node\LibreOffice",
        r"SOFTWARE\WOW6432Node\LibreOffice\UNO\InstallPath",
    ] {
        let Ok(key) = hklm.open_subkey(key_name) else {
            continue;
        };
        for value_name in ["InstallPath", "Path", ""] {
            if let Ok(value) = key.get_value::<String, _>(value_name) {
                if let Some(path) = executable_from_install_path(&value, "soffice.exe") {
                    return Some(path);
                }
            }
        }
    }
    None
}

pub fn is_windows_apps_python_alias(path: &Path) -> bool {
    let normalized = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    let file_name = normalized.rsplit('/').next().unwrap_or_default();
    matches!(file_name, "python.exe" | "python3.exe")
        && normalized.contains("/microsoft/windowsapps/")
}

pub fn python_is_usable(path: &Path) -> bool {
    if is_windows_apps_python_alias(path) {
        return false;
    }
    background_command(path)
        .args([
            "-c",
            "import sys; raise SystemExit(0 if sys.version_info >= (3, 10) else 1)",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

pub fn python_has_pdf_to_docx_packages(path: &Path) -> bool {
    if is_windows_apps_python_alias(path) {
        return false;
    }
    background_command(path)
        .args([
            "-c",
            "import pymupdf; from pdf2docx import Converter; raise SystemExit(0)",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(target_os = "windows")]
fn windows_python_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for root in [
        env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|path| path.join("Programs/Python")),
        env::var_os("ProgramFiles")
            .map(PathBuf::from)
            .map(|path| path.join("Python")),
    ]
    .into_iter()
    .flatten()
    {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        candidates.extend(
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path().join("python.exe"))
                .filter(|path| path.is_file()),
        );
    }
    candidates.sort_by(|left, right| right.cmp(left));
    candidates.dedup();
    candidates
}

#[cfg(target_os = "windows")]
fn python_registry_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for hive in [
        RegKey::predef(HKEY_CURRENT_USER),
        RegKey::predef(HKEY_LOCAL_MACHINE),
    ] {
        for root_name in [
            r"SOFTWARE\Python\PythonCore",
            r"SOFTWARE\WOW6432Node\Python\PythonCore",
        ] {
            let Ok(root) = hive.open_subkey(root_name) else {
                continue;
            };
            for version in root.enum_keys().filter_map(Result::ok) {
                let Ok(install_path) = root.open_subkey(format!(r"{version}\InstallPath")) else {
                    continue;
                };
                if let Ok(value) = install_path.get_value::<String, _>("ExecutablePath") {
                    candidates.push(PathBuf::from(value));
                }
                if let Ok(value) = install_path.get_value::<String, _>("") {
                    candidates.push(PathBuf::from(value).join("python.exe"));
                }
            }
        }
    }
    candidates.retain(|path| path.is_file());
    candidates
}

#[cfg(not(target_os = "windows"))]
fn windows_python_candidates() -> Vec<PathBuf> {
    Vec::new()
}

pub fn find_python() -> Option<PathBuf> {
    if let Some(path) = env::var_os("LOCAL_FILE_CONVERTER_PYTHON").map(PathBuf::from) {
        if path.is_file() && python_is_usable(&path) {
            return Some(path);
        }
    }

    let names: &[&str] = if cfg!(windows) {
        &["python.exe", "python3.exe"]
    } else {
        &["python3", "python"]
    };
    if let Some(path) = first_usable_python(python_path_candidates(names), python_is_usable) {
        return Some(path);
    }

    #[cfg(target_os = "windows")]
    if let Some(path) = first_usable_python(python_registry_candidates(), python_is_usable) {
        return Some(path);
    }

    first_usable_python(windows_python_candidates(), python_is_usable)
}

fn python_path_candidates(file_names: &[&str]) -> Vec<PathBuf> {
    let Some(path) = env::var_os("PATH") else {
        return Vec::new();
    };
    env::split_paths(&path)
        .flat_map(|directory| file_names.iter().map(move |name| directory.join(name)))
        .filter(|candidate| candidate.is_file())
        .collect()
}

fn first_usable_python<I, F>(candidates: I, mut usable: F) -> Option<PathBuf>
where
    I: IntoIterator<Item = PathBuf>,
    F: FnMut(&Path) -> bool,
{
    candidates
        .into_iter()
        .find(|candidate| !is_windows_apps_python_alias(candidate) && usable(candidate))
}

pub fn find_pdf_to_docx_worker(resource_dir: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(path) = env::var_os("LOCAL_FILE_CONVERTER_PDF_TO_DOCX_WORKER").map(PathBuf::from) {
        if path.is_file() {
            return Some(path);
        }
    }

    let mut candidates = Vec::new();
    if let Some(directory) = resource_dir {
        candidates.push(directory.join("workers/pdf_to_docx.py"));
    }
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../workers/pdf_to_docx.py"));
    if let Ok(executable) = env::current_exe() {
        if let Some(directory) = executable.parent() {
            candidates.push(directory.join("workers/pdf_to_docx.py"));
        }
    }
    candidates.into_iter().find(|candidate| candidate.is_file())
}

pub fn find_pdf_to_docx_runtime(resource_dir: Option<PathBuf>) -> Option<PdfToDocxRuntime> {
    let python_path = find_python()?;
    let worker_path = find_pdf_to_docx_worker(resource_dir)?;
    python_has_pdf_to_docx_packages(&python_path).then_some(PdfToDocxRuntime {
        python_path,
        worker_path,
    })
}

fn bundled_ffmpeg_file(tool: &str) -> String {
    if cfg!(windows) {
        format!("{tool}.exe")
    } else {
        tool.to_string()
    }
}

fn find_ffmpeg_tool(tool: &str, resource_dir: Option<PathBuf>) -> Option<PathBuf> {
    let file_name = bundled_ffmpeg_file(tool);
    let environment_key = format!("LOCAL_FILE_CONVERTER_{}", tool.to_ascii_uppercase());
    if let Some(path) = env::var_os(environment_key).map(PathBuf::from) {
        if path.is_file() {
            return Some(path);
        }
    }

    let mut candidates = Vec::new();
    if let Some(directory) = resource_dir {
        candidates.push(directory.join("vendor/ffmpeg/bin").join(&file_name));
    }
    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../vendor/ffmpeg/bin")
            .join(&file_name),
    );
    if let Ok(executable) = env::current_exe() {
        if let Some(directory) = executable.parent() {
            candidates.push(directory.join("vendor/ffmpeg/bin").join(&file_name));
        }
    }

    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .or_else(|| find_on_path(&[&file_name]))
}

pub fn find_ffmpeg(resource_dir: Option<PathBuf>) -> Option<PathBuf> {
    find_ffmpeg_tool("ffmpeg", resource_dir)
}

pub fn find_ffprobe(resource_dir: Option<PathBuf>) -> Option<PathBuf> {
    find_ffmpeg_tool("ffprobe", resource_dir)
}

pub fn find_qpdf(resource_dir: Option<PathBuf>) -> Option<PathBuf> {
    let file_name = if cfg!(windows) { "qpdf.exe" } else { "qpdf" };
    if let Some(path) = env::var_os("LOCAL_FILE_CONVERTER_QPDF").map(PathBuf::from) {
        if path.is_file() {
            return Some(path);
        }
    }

    let mut candidates = Vec::new();
    if let Some(directory) = resource_dir {
        candidates.push(directory.join("vendor/qpdf/bin").join(file_name));
    }
    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../vendor/qpdf/bin")
            .join(file_name),
    );
    if let Ok(executable) = env::current_exe() {
        if let Some(directory) = executable.parent() {
            candidates.push(directory.join("vendor/qpdf/bin").join(file_name));
        }
    }

    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .or_else(|| find_on_path(&[file_name]))
}

pub fn find_pdfium(resource_dir: Option<PathBuf>) -> Option<PathBuf> {
    let file_name = if cfg!(windows) {
        "pdfium.dll"
    } else if cfg!(target_os = "macos") {
        "libpdfium.dylib"
    } else {
        "libpdfium.so"
    };
    if let Some(path) = env::var_os("LOCAL_FILE_CONVERTER_PDFIUM").map(PathBuf::from) {
        if path.is_file() {
            return Some(path);
        }
    }

    let mut candidates = Vec::new();
    if let Some(directory) = resource_dir {
        candidates.push(directory.join("vendor/pdfium/bin").join(file_name));
    }
    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../vendor/pdfium/bin")
            .join(file_name),
    );
    if let Ok(executable) = env::current_exe() {
        if let Some(directory) = executable.parent() {
            candidates.push(directory.join("vendor/pdfium/bin").join(file_name));
        }
    }

    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .or_else(|| find_on_path(&[file_name]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn libreoffice_detection_never_returns_a_missing_file() {
        if let Some(path) = find_libreoffice() {
            assert!(path.is_file());
        }
    }

    #[test]
    fn python_detection_never_returns_an_unusable_runtime() {
        if let Some(path) = find_python() {
            assert!(path.is_file());
            assert!(python_is_usable(&path));
        }
    }

    #[test]
    fn windows_apps_alias_is_skipped_before_trying_the_next_python() {
        let alias =
            PathBuf::from(r"C:\Users\Example\AppData\Local\Microsoft\WindowsApps\python.exe");
        let alias3 =
            PathBuf::from(r"C:\Users\Example\AppData\Local\Microsoft\WindowsApps\python3.exe");
        let real = PathBuf::from(r"C:\Python312\python.exe");
        let mut probed = Vec::new();

        let selected =
            first_usable_python([alias.clone(), alias3.clone(), real.clone()], |candidate| {
                probed.push(candidate.to_path_buf());
                candidate == real
            });

        assert!(is_windows_apps_python_alias(&alias));
        assert!(is_windows_apps_python_alias(&alias3));
        assert_eq!(selected, Some(real.clone()));
        assert_eq!(probed, vec![real]);
    }

    #[test]
    fn pdf_to_docx_worker_is_present_in_the_source_tree() {
        let path = find_pdf_to_docx_worker(None).expect("the worker script should be present");
        assert!(path.is_file());
    }

    #[test]
    fn bundled_ffmpeg_detection_returns_a_real_file() {
        if cfg!(windows) {
            let path = find_ffmpeg(None).expect("the Windows bundle should contain FFmpeg");
            assert!(path.is_file());
        } else if let Some(path) = find_ffmpeg(None) {
            assert!(path.is_file());
        }
    }

    #[test]
    fn bundled_qpdf_detection_returns_a_real_file() {
        if cfg!(windows) {
            let path = find_qpdf(None).expect("the Windows bundle should contain qpdf");
            assert!(path.is_file());
        } else if let Some(path) = find_qpdf(None) {
            assert!(path.is_file());
        }
    }

    #[test]
    fn bundled_pdfium_detection_returns_a_real_file() {
        if cfg!(windows) {
            let path = find_pdfium(None).expect("the Windows bundle should contain PDFium");
            assert!(path.is_file());
        } else if let Some(path) = find_pdfium(None) {
            assert!(path.is_file());
        }
    }
}

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

/// The bundled frozen worker runs directly; a Python runtime runs the worker script.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PdfToDocxRuntime {
    pub program: PathBuf,
    pub script: Option<PathBuf>,
}

impl PdfToDocxRuntime {
    pub fn python(python_path: PathBuf, worker_path: PathBuf) -> Self {
        Self {
            program: python_path,
            script: Some(worker_path),
        }
    }

    pub fn bundled(executable: PathBuf) -> Self {
        Self {
            program: executable,
            script: None,
        }
    }

    pub fn is_available(&self) -> bool {
        self.program.is_file() && self.script.as_deref().is_none_or(Path::is_file)
    }

    pub(crate) fn command(&self) -> Command {
        let mut command = background_command(&self.program);
        if let Some(script) = &self.script {
            command.arg("-B").arg(script);
        }
        command
    }
}

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Creates a child process without allocating a visible console window on Windows.
pub(crate) fn background_command<S: AsRef<OsStr>>(program: S) -> Command {
    #[cfg_attr(not(target_os = "windows"), allow(unused_mut))]
    let mut command = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;

        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// Directories searched after `PATH`; apps started from a desktop launcher often get a minimal `PATH`.
fn well_known_directories() -> Vec<PathBuf> {
    let mut directories: Vec<PathBuf> = if cfg!(target_os = "macos") {
        ["/opt/homebrew/bin", "/usr/local/bin", "/opt/local/bin"]
            .into_iter()
            .map(PathBuf::from)
            .collect()
    } else if cfg!(target_os = "linux") {
        [
            "/usr/local/bin",
            "/usr/bin",
            "/bin",
            "/snap/bin",
            "/home/linuxbrew/.linuxbrew/bin",
        ]
        .into_iter()
        .map(PathBuf::from)
        .collect()
    } else {
        Vec::new()
    };
    if !cfg!(target_os = "windows") {
        if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
            directories.push(home.join(".local/bin"));
        }
    }
    directories
}

fn search_directories() -> Vec<PathBuf> {
    let mut directories: Vec<PathBuf> = env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect())
        .unwrap_or_default();
    for directory in well_known_directories() {
        if !directories.contains(&directory) {
            directories.push(directory);
        }
    }
    directories
}

fn find_on_path(file_names: &[&str]) -> Option<PathBuf> {
    search_directories()
        .into_iter()
        .flat_map(|directory| file_names.iter().map(move |name| directory.join(name)))
        .find(|candidate| candidate.is_file())
}

fn bundled_candidates(resource_dir: Option<&Path>, relative: &str) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(directory) = resource_dir {
        candidates.push(directory.join(relative));
    }
    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(relative),
    );
    if let Ok(executable) = env::current_exe() {
        if let Some(directory) = executable.parent() {
            candidates.push(directory.join(relative));
        }
    }
    candidates
}

pub fn find_bundled_libreoffice(resource_dir: Option<&Path>) -> Option<PathBuf> {
    let relative = if cfg!(target_os = "macos") {
        "vendor/libreoffice/LibreOffice.app/Contents/MacOS/soffice"
    } else if cfg!(windows) {
        "vendor/libreoffice/program/soffice.com"
    } else {
        "vendor/libreoffice/program/soffice"
    };
    bundled_candidates(resource_dir, relative)
        .into_iter()
        .find(|candidate| candidate.is_file())
}

pub fn find_whisper_model(resource_dir: Option<&Path>) -> Option<PathBuf> {
    bundled_candidates(resource_dir, "vendor/whisper/ggml-base-q5_1.bin")
        .into_iter()
        .find(|candidate| candidate.is_file())
}

/// The text-to-speech runtime executable, when it and every voice file are present.
pub fn find_tts(resource_dir: Option<&Path>) -> Option<TtsPaths> {
    let executable = if cfg!(windows) {
        "vendor/sherpa-onnx/bin/sherpa-onnx-offline-tts.exe"
    } else {
        "vendor/sherpa-onnx/bin/sherpa-onnx-offline-tts"
    };
    let executable = bundled_candidates(resource_dir, executable)
        .into_iter()
        .find(|candidate| candidate.is_file())?;
    let voices_directory = bundled_candidates(resource_dir, "vendor/tts")
        .into_iter()
        .find(|directory| {
            directory.join("espeak-ng-data").is_dir()
                && ["en", "tr"].iter().all(|voice| {
                    ["model.onnx", "tokens.txt"]
                        .iter()
                        .all(|file| directory.join("voices").join(voice).join(file).is_file())
                })
        })?;
    Some(TtsPaths {
        executable,
        voices_directory,
    })
}

#[derive(Clone, Debug)]
pub struct TtsPaths {
    pub executable: PathBuf,
    pub voices_directory: PathBuf,
}

/// The directory holding the bundled background-removal and upscaling models.
pub fn find_image_ai_models(resource_dir: Option<&Path>) -> Option<PathBuf> {
    bundled_candidates(resource_dir, "vendor/image-ai")
        .into_iter()
        .find(|directory| {
            ["u2netp.onnx", "realesr-general-x4v3.onnx"]
                .iter()
                .all(|name| directory.join(name).is_file())
        })
}

/// The directory holding the bundled `eng`/`tur` models and the PDF font.
pub fn find_tessdata(resource_dir: Option<&Path>) -> Option<PathBuf> {
    bundled_candidates(resource_dir, "vendor/tesseract/tessdata/eng.traineddata")
        .into_iter()
        .find(|candidate| candidate.is_file())
        .and_then(|model| model.parent().map(Path::to_path_buf))
        .filter(|directory| {
            ["tur.traineddata", "pdf.ttf"]
                .iter()
                .all(|name| directory.join(name).is_file())
        })
}

pub fn find_bundled_pdf_to_docx(resource_dir: Option<&Path>) -> Option<PathBuf> {
    let relative = if cfg!(windows) {
        "vendor/pdf-to-docx/pdf-to-docx.exe"
    } else {
        "vendor/pdf-to-docx/pdf-to-docx"
    };
    bundled_candidates(resource_dir, relative)
        .into_iter()
        .find(|candidate| candidate.is_file())
}

pub fn find_libreoffice(resource_dir: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = find_bundled_libreoffice(resource_dir) {
        return Some(path);
    }
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
    let candidates = {
        let mut candidates = vec![PathBuf::from(
            "/Applications/LibreOffice.app/Contents/MacOS/soffice",
        )];
        if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
            candidates.push(home.join("Applications/LibreOffice.app/Contents/MacOS/soffice"));
        }
        candidates
    };

    #[cfg(target_os = "linux")]
    let candidates = {
        let mut candidates = vec![PathBuf::from("/usr/lib/libreoffice/program/soffice")];
        if let Ok(entries) = std::fs::read_dir("/opt") {
            let mut versioned: Vec<_> = entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    path.file_name()
                        .is_some_and(|name| name.to_string_lossy().starts_with("libreoffice"))
                })
                .map(|path| path.join("program/soffice"))
                .collect();
            versioned.sort_by(|left, right| right.cmp(left));
            candidates.extend(versioned);
        }
        candidates
    };

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
    search_directories()
        .into_iter()
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
    if let Some(executable) = find_bundled_pdf_to_docx(resource_dir.as_deref()) {
        return Some(PdfToDocxRuntime::bundled(executable));
    }
    let python_path = find_python()?;
    let worker_path = find_pdf_to_docx_worker(resource_dir)?;
    python_has_pdf_to_docx_packages(&python_path)
        .then(|| PdfToDocxRuntime::python(python_path, worker_path))
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
        if let Some(path) = find_libreoffice(None) {
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
    fn package_manager_directories_are_searched_after_path() {
        let directories = search_directories();
        for directory in well_known_directories() {
            assert!(directories.contains(&directory));
        }
        if cfg!(target_os = "macos") {
            assert!(directories.contains(&PathBuf::from("/opt/homebrew/bin")));
        }
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

use std::path::{Path, PathBuf};

/// Expands a leading `~` and requires the result to be absolute.
pub fn absolute(path: &str) -> Result<String, String> {
    let expanded = match path.strip_prefix('~') {
        Some(rest) if rest.is_empty() || rest.starts_with(['/', '\\']) => {
            let home = dirs::home_dir().ok_or("The home directory could not be determined")?;
            home.join(rest.trim_start_matches(['/', '\\']))
        }
        _ => PathBuf::from(path),
    };
    if !expanded.is_absolute() {
        return Err(format!("Path must be absolute: {path}"));
    }
    Ok(expanded.to_string_lossy().into_owned())
}

pub fn absolute_all(paths: &[String]) -> Result<Vec<String>, String> {
    paths.iter().map(|path| absolute(path)).collect()
}

/// Uses the given output path, or picks a free `<stem><label>.<extension>` next to the input.
pub fn output_file(
    output: Option<&str>,
    input: &str,
    label: &str,
    extension: &str,
) -> Result<String, String> {
    if let Some(output) = output {
        return absolute(output);
    }
    let input = Path::new(input);
    let directory = input
        .parent()
        .ok_or("The input file has no parent directory")?;
    let stem = input
        .file_stem()
        .ok_or("The input file has no name")?
        .to_string_lossy();
    Ok(next_free(directory, &format!("{stem}{label}"), extension)
        .to_string_lossy()
        .into_owned())
}

/// Uses the given output directory, or the directory containing the input.
pub fn output_directory(output: Option<&str>, input: &str) -> Result<String, String> {
    if let Some(output) = output {
        return absolute(output);
    }
    let input = Path::new(input);
    let directory = if input.is_dir() {
        input
    } else {
        input
            .parent()
            .ok_or("The input file has no parent directory")?
    };
    Ok(directory.to_string_lossy().into_owned())
}

fn next_free(directory: &Path, stem: &str, extension: &str) -> PathBuf {
    let mut suffix = 1_u32;
    loop {
        let name = if suffix == 1 {
            format!("{stem}.{extension}")
        } else {
            format!("{stem}-{suffix}.{extension}")
        };
        let candidate = directory.join(name);
        if !candidate.exists() {
            return candidate;
        }
        suffix += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths_are_rejected() {
        assert!(absolute("music/song.wav").is_err());
    }

    #[test]
    fn tilde_expands_to_home() {
        let home = dirs::home_dir().unwrap();
        assert_eq!(
            absolute("~/song.wav").unwrap(),
            home.join("song.wav").to_string_lossy()
        );
    }

    #[test]
    fn default_output_avoids_existing_files() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("song.wav");
        std::fs::write(&input, b"").unwrap();
        std::fs::write(directory.path().join("song.mp3"), b"").unwrap();

        let output = output_file(None, &input.to_string_lossy(), "", "mp3").unwrap();

        assert_eq!(
            output,
            directory.path().join("song-2.mp3").to_string_lossy()
        );
    }

    #[test]
    fn label_is_appended_to_the_stem() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("report.pdf");
        std::fs::write(&input, b"").unwrap();

        let output = output_file(None, &input.to_string_lossy(), "-rotated", "pdf").unwrap();

        assert_eq!(
            output,
            directory
                .path()
                .join("report-rotated.pdf")
                .to_string_lossy()
        );
    }
}

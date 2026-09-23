use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use crate::domain::file_entry::{DirectoryListing, EntryKind, FileEntry, FileOperationResult};

type FileResult<T> = Result<T, String>;

fn canonical_existing(path: &str) -> FileResult<PathBuf> {
    let requested = PathBuf::from(path);
    if !requested.is_absolute() {
        return Err("Only absolute filesystem paths are accepted.".into());
    }
    requested
        .canonicalize()
        .map_err(|error| format!("The selected path is unavailable: {error}"))
}

fn existing_directory(path: &str) -> FileResult<PathBuf> {
    let path = canonical_existing(path)?;
    if !path.is_dir() {
        return Err("The selected path is not a directory.".into());
    }
    Ok(path)
}

fn validate_leaf_name(name: &str) -> FileResult<&str> {
    let trimmed = name.trim();
    if trimmed.is_empty() || matches!(trimmed, "." | "..") {
        return Err("A non-empty file or folder name is required.".into());
    }
    if trimmed.contains(['/', '\\']) || Path::new(trimmed).components().count() != 1 {
        return Err("The name cannot contain path separators.".into());
    }
    Ok(trimmed)
}

fn is_hidden(name: &str, metadata: &fs::Metadata) -> bool {
    if name.starts_with('.') {
        return true;
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
        metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0
    }

    #[cfg(not(target_os = "windows"))]
    false
}

fn entry_from_path(path: PathBuf) -> FileResult<FileEntry> {
    let metadata = fs::symlink_metadata(&path)
        .map_err(|error| format!("Could not read metadata for {}: {error}", path.display()))?;
    let file_type = metadata.file_type();
    let kind = if file_type.is_symlink() {
        EntryKind::Symlink
    } else if file_type.is_dir() {
        EntryKind::Directory
    } else if file_type.is_file() {
        EntryKind::File
    } else {
        EntryKind::Other
    };
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    let extension = path
        .extension()
        .map(|value| value.to_string_lossy().to_ascii_lowercase());
    let modified_at = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_secs());

    Ok(FileEntry {
        hidden: is_hidden(&name, &metadata),
        name,
        path: path.display().to_string(),
        kind,
        size_bytes: metadata.is_file().then_some(metadata.len()),
        modified_at,
        extension,
    })
}

pub fn list_directory(path: &str) -> FileResult<DirectoryListing> {
    let path = existing_directory(path)?;
    let mut entries = fs::read_dir(&path)
        .map_err(|error| format!("Could not read {}: {error}", path.display()))?
        .filter_map(Result::ok)
        .filter_map(|entry| entry_from_path(entry.path()).ok())
        .collect::<Vec<_>>();

    entries.sort_by(|left, right| {
        let left_rank = if matches!(left.kind, EntryKind::Directory) {
            0
        } else {
            1
        };
        let right_rank = if matches!(right.kind, EntryKind::Directory) {
            0
        } else {
            1
        };
        left_rank
            .cmp(&right_rank)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });

    Ok(DirectoryListing {
        parent: path.parent().map(|value| value.display().to_string()),
        path: path.display().to_string(),
        entries,
    })
}

pub fn create_directory(parent_path: &str, name: &str) -> FileResult<FileOperationResult> {
    let parent = existing_directory(parent_path)?;
    let target = parent.join(validate_leaf_name(name)?);
    if target.exists() {
        return Err("An item with that name already exists.".into());
    }
    fs::create_dir(&target)
        .map_err(|error| format!("Could not create {}: {error}", target.display()))?;
    Ok(FileOperationResult {
        affected_paths: vec![target.display().to_string()],
    })
}

pub fn rename_entry(path: &str, new_name: &str) -> FileResult<FileOperationResult> {
    let source = canonical_existing(path)?;
    let parent = source
        .parent()
        .ok_or_else(|| "Filesystem roots cannot be renamed.".to_string())?;
    let target = parent.join(validate_leaf_name(new_name)?);
    if target.exists() {
        return Err("An item with that name already exists.".into());
    }
    fs::rename(&source, &target)
        .map_err(|error| format!("Could not rename {}: {error}", source.display()))?;
    Ok(FileOperationResult {
        affected_paths: vec![target.display().to_string()],
    })
}

fn unique_destination(destination: &Path, source: &Path) -> FileResult<PathBuf> {
    let name = source
        .file_name()
        .ok_or_else(|| "Filesystem roots cannot be copied or moved.".to_string())?;
    let target = destination.join(name);
    if target.exists() {
        return Err(format!("{} already exists.", target.display()));
    }
    Ok(target)
}

fn copy_entry(source: &Path, target: &Path) -> FileResult<()> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|error| format!("Could not read {}: {error}", source.display()))?;
    if metadata.file_type().is_symlink() {
        return Err("Copying symbolic links is not supported in version 1.".into());
    }
    if metadata.is_file() {
        fs::copy(source, target)
            .map(|_| ())
            .map_err(|error| format!("Could not copy {}: {error}", source.display()))
    } else if metadata.is_dir() {
        if target.starts_with(source) {
            return Err("A directory cannot be copied into itself.".into());
        }
        fs::create_dir(target)
            .map_err(|error| format!("Could not create {}: {error}", target.display()))?;
        for child in fs::read_dir(source)
            .map_err(|error| format!("Could not read {}: {error}", source.display()))?
        {
            let child = child.map_err(|error| error.to_string())?;
            copy_entry(&child.path(), &target.join(child.file_name()))?;
        }
        Ok(())
    } else {
        Err("This filesystem item type cannot be copied.".into())
    }
}

pub fn copy_entries(sources: &[String], destination: &str) -> FileResult<FileOperationResult> {
    let destination = existing_directory(destination)?;
    let mut affected_paths = Vec::with_capacity(sources.len());
    for source in sources {
        let source = canonical_existing(source)?;
        let target = unique_destination(&destination, &source)?;
        copy_entry(&source, &target)?;
        affected_paths.push(target.display().to_string());
    }
    Ok(FileOperationResult { affected_paths })
}

pub fn move_entries(sources: &[String], destination: &str) -> FileResult<FileOperationResult> {
    let destination = existing_directory(destination)?;
    let mut affected_paths = Vec::with_capacity(sources.len());
    for source in sources {
        let source = canonical_existing(source)?;
        let target = unique_destination(&destination, &source)?;
        if source.is_dir() && target.starts_with(&source) {
            return Err("A directory cannot be moved into itself.".into());
        }
        fs::rename(&source, &target)
            .map_err(|error| format!("Could not move {}: {error}", source.display()))?;
        affected_paths.push(target.display().to_string());
    }
    Ok(FileOperationResult { affected_paths })
}

fn reject_protected_root(path: &Path) -> FileResult<()> {
    if path.parent().is_none() {
        return Err("Filesystem roots are protected.".into());
    }
    for variable in ["USERPROFILE", "HOME"] {
        if let Some(home) = std::env::var_os(variable) {
            if PathBuf::from(home).canonicalize().ok().as_deref() == Some(path) {
                return Err("The user home directory is protected.".into());
            }
        }
    }
    Ok(())
}

pub fn trash_entries(paths: &[String]) -> FileResult<FileOperationResult> {
    let mut affected_paths = Vec::with_capacity(paths.len());
    for path in paths {
        let path = canonical_existing(path)?;
        reject_protected_root(&path)?;
        trash::delete(&path)
            .map_err(|error| format!("Could not move {} to trash: {error}", path.display()))?;
        affected_paths.push(path.display().to_string());
    }
    Ok(FileOperationResult { affected_paths })
}

pub fn delete_entries_permanently(
    paths: &[String],
    confirmed: bool,
) -> FileResult<FileOperationResult> {
    if !confirmed {
        return Err("Permanent deletion requires explicit confirmation.".into());
    }
    let mut affected_paths = Vec::with_capacity(paths.len());
    for path in paths {
        let path = canonical_existing(path)?;
        reject_protected_root(&path)?;
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            fs::remove_dir_all(&path)
                .map_err(|error| format!("Could not delete {}: {error}", path.display()))?;
        } else {
            fs::remove_file(&path)
                .map_err(|error| format!("Could not delete {}: {error}", path.display()))?;
        }
        affected_paths.push(path.display().to_string());
    }
    Ok(FileOperationResult { affected_paths })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_directories_before_files() {
        let root = tempfile::tempdir().expect("temporary directory");
        fs::write(root.path().join("a.txt"), "test").expect("fixture file");
        fs::create_dir(root.path().join("z-folder")).expect("fixture directory");

        let listing = list_directory(&root.path().display().to_string()).expect("listing");

        assert_eq!(listing.entries[0].name, "z-folder");
        assert_eq!(listing.entries[1].name, "a.txt");
    }

    #[test]
    fn refuses_to_overwrite_existing_items() {
        let root = tempfile::tempdir().expect("temporary directory");
        let source_dir = root.path().join("source");
        let destination_dir = root.path().join("destination");
        fs::create_dir(&source_dir).expect("source directory");
        fs::create_dir(&destination_dir).expect("destination directory");
        fs::write(source_dir.join("note.txt"), "source").expect("source file");
        fs::write(destination_dir.join("note.txt"), "existing").expect("existing file");

        let result = copy_entries(
            &[source_dir.join("note.txt").display().to_string()],
            &destination_dir.display().to_string(),
        );

        assert!(result.is_err());
        assert_eq!(
            fs::read_to_string(destination_dir.join("note.txt")).expect("existing contents"),
            "existing"
        );
    }

    #[test]
    fn permanent_deletion_requires_confirmation() {
        let root = tempfile::tempdir().expect("temporary directory");
        let file = root.path().join("keep.txt");
        fs::write(&file, "keep").expect("fixture file");

        let result = delete_entries_permanently(&[file.display().to_string()], false);

        assert!(result.is_err());
        assert!(file.exists());
    }
}

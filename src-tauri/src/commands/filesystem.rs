use crate::{
    application::file_manager,
    domain::file_entry::{DirectoryListing, FileOperationResult},
};

#[tauri::command]
pub fn list_directory(path: String) -> Result<DirectoryListing, String> {
    file_manager::list_directory(&path)
}

#[tauri::command]
pub fn create_directory(parent_path: String, name: String) -> Result<FileOperationResult, String> {
    file_manager::create_directory(&parent_path, &name)
}

#[tauri::command]
pub fn rename_entry(path: String, new_name: String) -> Result<FileOperationResult, String> {
    file_manager::rename_entry(&path, &new_name)
}

#[tauri::command]
pub fn copy_entries(
    sources: Vec<String>,
    destination: String,
) -> Result<FileOperationResult, String> {
    file_manager::copy_entries(&sources, &destination)
}

#[tauri::command]
pub fn move_entries(
    sources: Vec<String>,
    destination: String,
) -> Result<FileOperationResult, String> {
    file_manager::move_entries(&sources, &destination)
}

#[tauri::command]
pub fn trash_entries(paths: Vec<String>) -> Result<FileOperationResult, String> {
    file_manager::trash_entries(&paths)
}

#[tauri::command]
pub fn delete_entries_permanently(
    paths: Vec<String>,
    confirmed: bool,
) -> Result<FileOperationResult, String> {
    file_manager::delete_entries_permanently(&paths, confirmed)
}

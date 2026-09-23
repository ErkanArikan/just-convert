use std::{path::PathBuf, sync::Arc};

use tauri::State;

use crate::application::dependency_service::{DependencyService, DependencyStatuses};

#[tauri::command]
pub fn set_dependency_override(
    service: State<'_, Arc<DependencyService>>,
    dependency: String,
    path: Option<String>,
) -> Result<DependencyStatuses, String> {
    service.set_override(&dependency, path.map(PathBuf::from))?;
    Ok(service.statuses())
}

#[tauri::command]
pub fn get_dependency_statuses(service: State<'_, Arc<DependencyService>>) -> DependencyStatuses {
    service.statuses()
}

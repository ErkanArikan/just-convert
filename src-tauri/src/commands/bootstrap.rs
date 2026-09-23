use serde::Serialize;
use std::sync::Arc;
use tauri::State;

use crate::{
    application::{
        converter_registry,
        dependency_service::{DependencyService, DependencyStatuses},
    },
    domain::converter::ConverterCapability,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppBootstrap {
    app_version: &'static str,
    platform: &'static str,
    converters: Vec<ConverterCapability>,
    dependencies: DependencyStatuses,
}

#[tauri::command]
pub fn get_app_bootstrap(service: State<'_, Arc<DependencyService>>) -> AppBootstrap {
    let dependencies = service.statuses();
    AppBootstrap {
        app_version: env!("CARGO_PKG_VERSION"),
        platform: std::env::consts::OS,
        converters: converter_registry::capabilities_with_dependencies(
            dependencies.libreoffice.ready,
            dependencies.python.ready,
        ),
        dependencies,
    }
}

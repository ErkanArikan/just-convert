pub mod application;
mod commands;
pub mod domain;
mod platform;

use std::sync::Arc;

use application::{
    dependency_service::DependencyService,
    job_service::{EnginePaths, JobService},
};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let resource_directory = app.path().resource_dir().ok();
            let app_data_directory = app.path().app_data_dir()?;
            let dependencies = Arc::new(DependencyService::new(
                &app_data_directory,
                resource_directory.clone(),
            )?);
            let service = Arc::new(JobService::new(
                app_data_directory,
                EnginePaths {
                    ffmpeg_path: platform::find_ffmpeg(resource_directory.clone()),
                    ffprobe_path: platform::find_ffprobe(resource_directory.clone()),
                    qpdf_path: platform::find_qpdf(resource_directory.clone()),
                    pdfium_path: platform::find_pdfium(resource_directory.clone()),
                    dependency_service: Some(Arc::clone(&dependencies)),
                    pdf_to_docx_worker_path: platform::find_pdf_to_docx_worker(resource_directory),
                    ..EnginePaths::default()
                },
            )?);
            app.manage(dependencies);
            app.manage(Arc::clone(&service));
            service.start();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap::get_app_bootstrap,
            commands::dependencies::get_dependency_statuses,
            commands::dependencies::set_dependency_override,
            commands::filesystem::list_directory,
            commands::filesystem::create_directory,
            commands::filesystem::rename_entry,
            commands::filesystem::copy_entries,
            commands::filesystem::move_entries,
            commands::filesystem::trash_entries,
            commands::filesystem::delete_entries_permanently,
            commands::jobs::enqueue_audio_conversion,
            commands::jobs::enqueue_audio_folder_conversion,
            commands::jobs::enqueue_pdf_operation,
            commands::jobs::enqueue_images_to_pdf,
            commands::jobs::enqueue_image_folder_to_pdf,
            commands::jobs::enqueue_pdf_to_images,
            commands::jobs::enqueue_pdf_folder_to_images,
            commands::jobs::enqueue_office_to_pdf,
            commands::jobs::enqueue_office_folder_to_pdf,
            commands::jobs::enqueue_pdf_to_docx,
            commands::jobs::enqueue_pdf_folder_to_docx,
            commands::jobs::get_pdf_page_count,
            commands::jobs::list_jobs,
            commands::jobs::cancel_job,
            commands::jobs::delete_job_history,
            commands::jobs::clear_job_history,
            commands::jobs::clear_job_history_section,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Just Convert");
}

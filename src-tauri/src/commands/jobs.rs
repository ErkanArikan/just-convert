use std::sync::Arc;

use tauri::State;

use crate::{
    application::job_service::JobService,
    domain::job::{AudioFormat, ImageFolderMode, Job, PdfImageFormat, PdfOperation},
};

#[tauri::command]
pub fn enqueue_audio_conversion(
    service: State<'_, Arc<JobService>>,
    input_path: String,
    output_path: String,
    format: AudioFormat,
) -> Result<Job, String> {
    service.enqueue_audio(input_path, output_path, format)
}

#[tauri::command]
pub fn enqueue_audio_folder_conversion(
    service: State<'_, Arc<JobService>>,
    input_directory: String,
    output_directory: String,
    format: AudioFormat,
) -> Result<Vec<Job>, String> {
    service.enqueue_audio_folder(input_directory, output_directory, format)
}

#[tauri::command]
pub fn list_jobs(service: State<'_, Arc<JobService>>) -> Result<Vec<Job>, String> {
    service.list_jobs()
}

#[tauri::command]
pub fn enqueue_pdf_operation(
    service: State<'_, Arc<JobService>>,
    operation: PdfOperation,
) -> Result<Job, String> {
    service.enqueue_pdf(operation)
}

#[tauri::command]
pub fn enqueue_images_to_pdf(
    service: State<'_, Arc<JobService>>,
    input_paths: Vec<String>,
    output_path: String,
) -> Result<Job, String> {
    service.enqueue_images_to_pdf(input_paths, output_path)
}

#[tauri::command]
pub fn enqueue_image_folder_to_pdf(
    service: State<'_, Arc<JobService>>,
    input_directory: String,
    output_directory: String,
    mode: ImageFolderMode,
) -> Result<Vec<Job>, String> {
    service.enqueue_image_folder_to_pdf(input_directory, output_directory, mode)
}

#[tauri::command]
pub fn enqueue_pdf_to_images(
    service: State<'_, Arc<JobService>>,
    input_path: String,
    output_directory: String,
    format: PdfImageFormat,
    dpi: u16,
) -> Result<Job, String> {
    service.enqueue_pdf_to_images(input_path, output_directory, format, dpi)
}

#[tauri::command]
pub fn enqueue_pdf_folder_to_images(
    service: State<'_, Arc<JobService>>,
    input_directory: String,
    output_directory: String,
    format: PdfImageFormat,
    dpi: u16,
) -> Result<Vec<Job>, String> {
    service.enqueue_pdf_folder_to_images(input_directory, output_directory, format, dpi)
}

#[tauri::command]
pub fn enqueue_office_to_pdf(
    service: State<'_, Arc<JobService>>,
    input_path: String,
    output_path: String,
) -> Result<Job, String> {
    service.enqueue_office_to_pdf(input_path, output_path)
}

#[tauri::command]
pub fn enqueue_office_folder_to_pdf(
    service: State<'_, Arc<JobService>>,
    input_directory: String,
    output_directory: String,
) -> Result<Vec<Job>, String> {
    service.enqueue_office_folder_to_pdf(input_directory, output_directory)
}

#[tauri::command]
pub fn enqueue_pdf_to_docx(
    service: State<'_, Arc<JobService>>,
    input_path: String,
    output_path: String,
) -> Result<Job, String> {
    service.enqueue_pdf_to_docx(input_path, output_path)
}

#[tauri::command]
pub fn enqueue_pdf_folder_to_docx(
    service: State<'_, Arc<JobService>>,
    input_directory: String,
    output_directory: String,
) -> Result<Vec<Job>, String> {
    service.enqueue_pdf_folder_to_docx(input_directory, output_directory)
}

#[tauri::command]
pub fn get_pdf_page_count(
    service: State<'_, Arc<JobService>>,
    input_path: String,
) -> Result<u32, String> {
    service.pdf_page_count(&input_path)
}

#[tauri::command]
pub fn cancel_job(service: State<'_, Arc<JobService>>, id: String) -> Result<Job, String> {
    service.cancel(&id)
}

#[tauri::command]
pub fn delete_job_history(service: State<'_, Arc<JobService>>, id: String) -> Result<(), String> {
    service.delete_history_entry(&id)
}

#[tauri::command]
pub fn clear_job_history(service: State<'_, Arc<JobService>>) -> Result<usize, String> {
    service.clear_history()
}

#[tauri::command]
pub fn clear_job_history_section(
    service: State<'_, Arc<JobService>>,
    section: String,
) -> Result<usize, String> {
    service.clear_history_section(&section)
}

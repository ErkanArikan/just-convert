use std::{
    cmp::Reverse,
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    application::{
        audio_converter::{self, AudioConversion},
        dependency_service::DependencyService,
        image_converter, job_history,
        job_queue::JobQueue,
        office_converter, pdf_converter, pdf_docx_converter, pdf_image_converter,
    },
    domain::job::{
        AudioFormat, ExecutionOutcome, ImageFolderMode, Job, JobError, JobKind, JobStatus,
        PdfImageFormat, PdfOperation,
    },
};

static NEXT_JOB_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Default)]
pub struct EnginePaths {
    pub ffmpeg_path: Option<PathBuf>,
    pub ffprobe_path: Option<PathBuf>,
    pub qpdf_path: Option<PathBuf>,
    pub pdfium_path: Option<PathBuf>,
    pub libreoffice_path: Option<PathBuf>,
    pub python_path: Option<PathBuf>,
    pub pdf_to_docx_worker_path: Option<PathBuf>,
    pub dependency_service: Option<Arc<DependencyService>>,
}

struct ServiceState {
    queue: JobQueue,
    jobs: Vec<Job>,
    cancellations: HashMap<String, Arc<AtomicBool>>,
    worker_running: bool,
}

pub struct JobService {
    state: Mutex<ServiceState>,
    history_database: PathBuf,
    ffmpeg_path: Option<PathBuf>,
    ffprobe_path: Option<PathBuf>,
    qpdf_path: Option<PathBuf>,
    pdfium_path: Option<PathBuf>,
    libreoffice_path: Option<PathBuf>,
    python_path: Option<PathBuf>,
    pdf_to_docx_worker_path: Option<PathBuf>,
    dependency_service: Option<Arc<DependencyService>>,
}

impl JobService {
    pub fn new(data_directory: PathBuf, engines: EnginePaths) -> Result<Self, String> {
        fs::create_dir_all(&data_directory)
            .map_err(|error| format!("Application data directory could not be created: {error}"))?;
        let (history_database, mut jobs) = job_history::initialize(&data_directory)?;
        let mut queue = JobQueue::new(1);
        let now = unix_timestamp();

        for job in &mut jobs {
            match job.status {
                JobStatus::Queued => queue.enqueue(job.clone()),
                JobStatus::Running => {
                    job.status = JobStatus::Failed;
                    job.finished_at = Some(now);
                    job.error = Some(JobError {
                        code: "interrupted".into(),
                        message: "The application closed before this job finished".into(),
                    });
                }
                _ => {}
            }
        }

        let service = Self {
            state: Mutex::new(ServiceState {
                queue,
                jobs,
                cancellations: HashMap::new(),
                worker_running: false,
            }),
            history_database,
            ffmpeg_path: engines.ffmpeg_path,
            ffprobe_path: engines.ffprobe_path,
            qpdf_path: engines.qpdf_path,
            pdfium_path: engines.pdfium_path,
            libreoffice_path: engines.libreoffice_path,
            python_path: engines.python_path,
            pdf_to_docx_worker_path: engines.pdf_to_docx_worker_path,
            dependency_service: engines.dependency_service,
        };
        {
            let state = service.state.lock().map_err(|_| "Job queue lock failed")?;
            service.persist(&state)?;
        }
        Ok(service)
    }

    pub fn start(self: &Arc<Self>) {
        self.start_worker_if_needed();
    }

    pub fn list_jobs(&self) -> Result<Vec<Job>, String> {
        let state = self.state.lock().map_err(|_| "Job queue lock failed")?;
        let mut jobs = state.jobs.clone();
        jobs.sort_by_key(|job| Reverse(job.created_at));
        Ok(jobs)
    }

    pub fn enqueue_audio(
        self: &Arc<Self>,
        input_path: String,
        output_path: String,
        format: AudioFormat,
    ) -> Result<Job, String> {
        if self.ffmpeg_path.is_none() {
            return Err("The bundled FFmpeg executable is unavailable".into());
        }
        audio_converter::validate_request(Path::new(&input_path), Path::new(&output_path), format)?;

        let now = unix_timestamp();
        let id = format!("{now}-{}", NEXT_JOB_ID.fetch_add(1, Ordering::Relaxed));
        let job = Job::queued_audio(id, input_path, output_path, format, now);
        {
            let mut state = self.state.lock().map_err(|_| "Job queue lock failed")?;
            state.queue.enqueue(job.clone());
            state.jobs.push(job.clone());
            self.persist(&state)?;
        }
        self.start_worker_if_needed();
        Ok(job)
    }

    pub fn enqueue_audio_folder(
        self: &Arc<Self>,
        input_directory: String,
        output_directory: String,
        format: AudioFormat,
    ) -> Result<Vec<Job>, String> {
        if self.ffmpeg_path.is_none() {
            return Err("The bundled FFmpeg executable is unavailable".into());
        }

        let conversions = plan_audio_folder_conversion(
            Path::new(&input_directory),
            Path::new(&output_directory),
            format,
        )?;
        let now = unix_timestamp();
        let jobs = conversions
            .into_iter()
            .map(|(input_path, output_path)| {
                let id = format!("{now}-{}", NEXT_JOB_ID.fetch_add(1, Ordering::Relaxed));
                Job::queued_audio(
                    id,
                    input_path.to_string_lossy().into_owned(),
                    output_path.to_string_lossy().into_owned(),
                    format,
                    now,
                )
            })
            .collect::<Vec<_>>();

        {
            let mut state = self.state.lock().map_err(|_| "Job queue lock failed")?;
            for job in &jobs {
                state.queue.enqueue(job.clone());
                state.jobs.push(job.clone());
            }
            self.persist(&state)?;
        }
        self.start_worker_if_needed();
        Ok(jobs)
    }

    pub fn enqueue_pdf(self: &Arc<Self>, operation: PdfOperation) -> Result<Job, String> {
        if self.qpdf_path.is_none() {
            return Err("The bundled qpdf executable is unavailable".into());
        }
        pdf_converter::validate_request(&operation)?;

        let now = unix_timestamp();
        let id = format!("{now}-{}", NEXT_JOB_ID.fetch_add(1, Ordering::Relaxed));
        let job = Job::queued_pdf(id, operation, now);
        {
            let mut state = self.state.lock().map_err(|_| "Job queue lock failed")?;
            state.queue.enqueue(job.clone());
            state.jobs.push(job.clone());
            self.persist(&state)?;
        }
        self.start_worker_if_needed();
        Ok(job)
    }

    pub fn enqueue_images_to_pdf(
        self: &Arc<Self>,
        input_paths: Vec<String>,
        output_path: String,
    ) -> Result<Job, String> {
        let qpdf_path = self
            .qpdf_path
            .as_deref()
            .ok_or("The bundled qpdf executable is unavailable")?;
        if !qpdf_path.is_file() {
            return Err("The bundled qpdf executable is unavailable".into());
        }
        image_converter::validate_request(&input_paths, Path::new(&output_path))?;

        let now = unix_timestamp();
        let id = format!("{now}-{}", NEXT_JOB_ID.fetch_add(1, Ordering::Relaxed));
        let job = Job::queued_images_to_pdf(id, input_paths, output_path, now);
        {
            let mut state = self.state.lock().map_err(|_| "Job queue lock failed")?;
            state.queue.enqueue(job.clone());
            state.jobs.push(job.clone());
            self.persist(&state)?;
        }
        self.start_worker_if_needed();
        Ok(job)
    }

    pub fn enqueue_image_folder_to_pdf(
        self: &Arc<Self>,
        input_directory: String,
        output_directory: String,
        mode: ImageFolderMode,
    ) -> Result<Vec<Job>, String> {
        let qpdf_path = self
            .qpdf_path
            .as_deref()
            .ok_or("The bundled qpdf executable is unavailable")?;
        if !qpdf_path.is_file() {
            return Err("The bundled qpdf executable is unavailable".into());
        }
        let now = unix_timestamp();
        let jobs = plan_image_folder_to_pdf(
            Path::new(&input_directory),
            Path::new(&output_directory),
            mode,
            now,
        )?;
        self.enqueue_batch_jobs(jobs)
    }

    pub fn pdf_page_count(&self, input_path: &str) -> Result<u32, String> {
        let qpdf_path = self
            .qpdf_path
            .as_deref()
            .ok_or("The bundled qpdf executable is unavailable")?;
        pdf_converter::inspect_page_count(qpdf_path, Path::new(input_path))
    }

    pub fn enqueue_pdf_to_images(
        self: &Arc<Self>,
        input_path: String,
        output_directory: String,
        format: PdfImageFormat,
        dpi: u16,
    ) -> Result<Job, String> {
        let pdfium_path = self
            .pdfium_path
            .as_deref()
            .ok_or("The bundled PDFium library is unavailable")?;
        if !pdfium_path.is_file() {
            return Err("The bundled PDFium library is unavailable".into());
        }
        pdf_image_converter::validate_request(
            Path::new(&input_path),
            Path::new(&output_directory),
            format,
            dpi,
        )?;

        let now = unix_timestamp();
        let id = format!("{now}-{}", NEXT_JOB_ID.fetch_add(1, Ordering::Relaxed));
        let job = Job::queued_pdf_to_images(id, input_path, output_directory, format, dpi, now);
        {
            let mut state = self.state.lock().map_err(|_| "Job queue lock failed")?;
            state.queue.enqueue(job.clone());
            state.jobs.push(job.clone());
            self.persist(&state)?;
        }
        self.start_worker_if_needed();
        Ok(job)
    }

    pub fn enqueue_pdf_folder_to_images(
        self: &Arc<Self>,
        input_directory: String,
        output_directory: String,
        format: PdfImageFormat,
        dpi: u16,
    ) -> Result<Vec<Job>, String> {
        let pdfium_path = self
            .pdfium_path
            .as_deref()
            .ok_or("The bundled PDFium library is unavailable")?;
        if !pdfium_path.is_file() {
            return Err("The bundled PDFium library is unavailable".into());
        }
        validate_folder_pair(Path::new(&input_directory), Path::new(&output_directory))?;
        let inputs = collect_folder_files(Path::new(&input_directory), |path| {
            has_extension(path, &["pdf"])
        })?;
        if inputs.is_empty() {
            return Err("No PDF files were found in the selected folder".into());
        }

        let mut reserved = HashSet::new();
        let now = unix_timestamp();
        let mut jobs = Vec::with_capacity(inputs.len());
        for input_path in inputs {
            let stem = file_stem(&input_path)?;
            let output_path =
                next_available_directory(Path::new(&output_directory), stem, &mut reserved);
            fs::create_dir(&output_path).map_err(|error| {
                format!(
                    "PDF image output folder could not be created ({}): {error}",
                    output_path.display()
                )
            })?;
            pdf_image_converter::validate_request(&input_path, &output_path, format, dpi)?;
            let id = format!("{now}-{}", NEXT_JOB_ID.fetch_add(1, Ordering::Relaxed));
            jobs.push(Job::queued_pdf_to_images(
                id,
                input_path.to_string_lossy().into_owned(),
                output_path.to_string_lossy().into_owned(),
                format,
                dpi,
                now,
            ));
        }
        self.enqueue_batch_jobs(jobs)
    }

    pub fn enqueue_office_to_pdf(
        self: &Arc<Self>,
        input_path: String,
        output_path: String,
    ) -> Result<Job, String> {
        let libreoffice_path = self
            .resolved_libreoffice_path()
            .ok_or("LibreOffice is not installed")?;
        if !libreoffice_path.is_file() || self.qpdf_path.is_none() {
            return Err("LibreOffice or the bundled PDF validator is unavailable".into());
        }
        office_converter::validate_request(Path::new(&input_path), Path::new(&output_path))?;

        let now = unix_timestamp();
        let id = format!("{now}-{}", NEXT_JOB_ID.fetch_add(1, Ordering::Relaxed));
        let job = Job::queued_office_to_pdf(id, input_path, output_path, now);
        {
            let mut state = self.state.lock().map_err(|_| "Job queue lock failed")?;
            state.queue.enqueue(job.clone());
            state.jobs.push(job.clone());
            self.persist(&state)?;
        }
        self.start_worker_if_needed();
        Ok(job)
    }

    pub fn enqueue_office_folder_to_pdf(
        self: &Arc<Self>,
        input_directory: String,
        output_directory: String,
    ) -> Result<Vec<Job>, String> {
        let libreoffice_path = self
            .resolved_libreoffice_path()
            .ok_or("LibreOffice is not installed")?;
        if !libreoffice_path.is_file() || self.qpdf_path.is_none() {
            return Err("LibreOffice or the bundled PDF validator is unavailable".into());
        }
        validate_folder_pair(Path::new(&input_directory), Path::new(&output_directory))?;
        let inputs = collect_folder_files(
            Path::new(&input_directory),
            office_converter::is_supported_input,
        )?;
        if inputs.is_empty() {
            return Err(
                "No supported Office files were found in the selected folder (DOCX, XLSX, or PPTX)"
                    .into(),
            );
        }

        let now = unix_timestamp();
        let mut reserved = HashSet::new();
        let mut jobs = Vec::with_capacity(inputs.len());
        for input_path in inputs {
            let output_path = next_available_output(
                Path::new(&output_directory),
                file_stem(&input_path)?,
                "pdf",
                &mut reserved,
            );
            office_converter::validate_request(&input_path, &output_path)?;
            let id = format!("{now}-{}", NEXT_JOB_ID.fetch_add(1, Ordering::Relaxed));
            jobs.push(Job::queued_office_to_pdf(
                id,
                input_path.to_string_lossy().into_owned(),
                output_path.to_string_lossy().into_owned(),
                now,
            ));
        }
        self.enqueue_batch_jobs(jobs)
    }

    pub fn enqueue_pdf_to_docx(
        self: &Arc<Self>,
        input_path: String,
        output_path: String,
    ) -> Result<Job, String> {
        let (python_path, worker_path) = self
            .resolved_pdf_to_docx_runtime()
            .ok_or("Python, PyMuPDF, and pdf2docx are required")?;
        if !python_path.is_file() || !worker_path.is_file() {
            return Err("Python PDF-to-Word dependencies are unavailable".into());
        }
        pdf_docx_converter::validate_request(Path::new(&input_path), Path::new(&output_path))?;

        let now = unix_timestamp();
        let id = format!("{now}-{}", NEXT_JOB_ID.fetch_add(1, Ordering::Relaxed));
        let job = Job::queued_pdf_to_docx(id, input_path, output_path, now);
        {
            let mut state = self.state.lock().map_err(|_| "Job queue lock failed")?;
            state.queue.enqueue(job.clone());
            state.jobs.push(job.clone());
            self.persist(&state)?;
        }
        self.start_worker_if_needed();
        Ok(job)
    }

    pub fn enqueue_pdf_folder_to_docx(
        self: &Arc<Self>,
        input_directory: String,
        output_directory: String,
    ) -> Result<Vec<Job>, String> {
        let (python_path, worker_path) = self
            .resolved_pdf_to_docx_runtime()
            .ok_or("Python, PyMuPDF, and pdf2docx are required")?;
        if !python_path.is_file() || !worker_path.is_file() {
            return Err("Python PDF-to-Word dependencies are unavailable".into());
        }
        validate_folder_pair(Path::new(&input_directory), Path::new(&output_directory))?;
        let inputs = collect_folder_files(Path::new(&input_directory), |path| {
            has_extension(path, &["pdf"])
        })?;
        if inputs.is_empty() {
            return Err("No PDF files were found in the selected folder".into());
        }

        let now = unix_timestamp();
        let mut reserved = HashSet::new();
        let mut jobs = Vec::with_capacity(inputs.len());
        for input_path in inputs {
            let output_path = next_available_output(
                Path::new(&output_directory),
                file_stem(&input_path)?,
                "docx",
                &mut reserved,
            );
            pdf_docx_converter::validate_request(&input_path, &output_path)?;
            let id = format!("{now}-{}", NEXT_JOB_ID.fetch_add(1, Ordering::Relaxed));
            jobs.push(Job::queued_pdf_to_docx(
                id,
                input_path.to_string_lossy().into_owned(),
                output_path.to_string_lossy().into_owned(),
                now,
            ));
        }
        self.enqueue_batch_jobs(jobs)
    }

    fn enqueue_batch_jobs(self: &Arc<Self>, jobs: Vec<Job>) -> Result<Vec<Job>, String> {
        {
            let mut state = self.state.lock().map_err(|_| "Job queue lock failed")?;
            for job in &jobs {
                state.queue.enqueue(job.clone());
                state.jobs.push(job.clone());
            }
            self.persist(&state)?;
        }
        self.start_worker_if_needed();
        Ok(jobs)
    }

    pub fn cancel(&self, id: &str) -> Result<Job, String> {
        let mut state = self.state.lock().map_err(|_| "Job queue lock failed")?;
        let index = state
            .jobs
            .iter()
            .position(|job| job.id == id)
            .ok_or("Job not found")?;

        match state.jobs[index].status {
            JobStatus::Queued => {
                state.queue.remove(id);
                state.jobs[index].status = JobStatus::Cancelled;
                state.jobs[index].finished_at = Some(unix_timestamp());
            }
            JobStatus::Running => {
                let cancellation = state
                    .cancellations
                    .get(id)
                    .ok_or("Running job cancellation handle unavailable")?;
                cancellation.store(true, Ordering::Relaxed);
            }
            _ => return Err("Only queued or running jobs can be cancelled".into()),
        }
        let job = state.jobs[index].clone();
        self.persist(&state)?;
        Ok(job)
    }

    pub fn delete_history_entry(&self, id: &str) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|_| "Job queue lock failed")?;
        let index = state
            .jobs
            .iter()
            .position(|job| job.id == id)
            .ok_or("Job not found")?;
        if !is_history_entry(&state.jobs[index].status) {
            return Err("Active jobs cannot be deleted; cancel the job first".into());
        }
        let removed = state.jobs.remove(index);
        if let Err(error) = self.persist(&state) {
            state.jobs.insert(index, removed);
            return Err(error);
        }
        Ok(())
    }

    pub fn clear_history(&self) -> Result<usize, String> {
        let mut state = self.state.lock().map_err(|_| "Job queue lock failed")?;
        let previous_jobs = state.jobs.clone();
        let previous_len = previous_jobs.len();
        state
            .jobs
            .retain(|job| !matches!(job.status, JobStatus::Completed | JobStatus::Cancelled));
        let removed = previous_len - state.jobs.len();
        if removed > 0 {
            if let Err(error) = self.persist(&state) {
                state.jobs = previous_jobs;
                return Err(error);
            }
        }
        Ok(removed)
    }

    pub fn clear_history_section(&self, section: &str) -> Result<usize, String> {
        let mut state = self.state.lock().map_err(|_| "Job queue lock failed")?;
        let previous_jobs = state.jobs.clone();
        let previous_len = previous_jobs.len();
        state.jobs.retain(|job| match section {
            "completed" => job.status != JobStatus::Completed,
            "failed" => !matches!(job.status, JobStatus::Failed | JobStatus::Cancelled),
            _ => true,
        });
        if !matches!(section, "completed" | "failed") {
            return Err("Unknown job history section".into());
        }
        let removed = previous_len - state.jobs.len();
        if removed > 0 {
            if let Err(error) = self.persist(&state) {
                state.jobs = previous_jobs;
                return Err(error);
            }
        }
        Ok(removed)
    }

    fn start_worker_if_needed(self: &Arc<Self>) {
        let should_start = {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            if state.worker_running || state.queue.pending_count() == 0 {
                false
            } else {
                state.worker_running = true;
                true
            }
        };

        if should_start {
            let service = Arc::clone(self);
            thread::spawn(move || service.worker_loop());
        }
    }

    fn worker_loop(self: Arc<Self>) {
        loop {
            let next = {
                let Ok(mut state) = self.state.lock() else {
                    return;
                };
                let Some(next) = state.queue.take_next() else {
                    state.worker_running = false;
                    return;
                };
                let cancellation = Arc::new(AtomicBool::new(false));
                state
                    .cancellations
                    .insert(next.id.clone(), Arc::clone(&cancellation));
                if let Some(job) = state.jobs.iter_mut().find(|job| job.id == next.id) {
                    job.status = JobStatus::Running;
                    job.started_at = Some(unix_timestamp());
                    job.progress = 0;
                }
                let _ = self.persist(&state);
                (next, cancellation)
            };

            let (job, cancellation) = next;
            let result = self.execute(&job, Arc::clone(&cancellation));
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            state.queue.finish_active();
            state.cancellations.remove(&job.id);
            if let Some(stored_job) = state.jobs.iter_mut().find(|item| item.id == job.id) {
                stored_job.finished_at = Some(unix_timestamp());
                match result {
                    Ok(ExecutionOutcome::Completed) => {
                        stored_job.status = JobStatus::Completed;
                        stored_job.progress = 100;
                        stored_job.error = None;
                    }
                    Ok(ExecutionOutcome::Cancelled) => {
                        stored_job.status = JobStatus::Cancelled;
                        stored_job.error = None;
                    }
                    Err(message) => {
                        stored_job.status = JobStatus::Failed;
                        stored_job.error = Some(JobError {
                            code: "conversion_failed".into(),
                            message,
                        });
                    }
                }
            }
            let _ = self.persist(&state);
        }
    }

    fn execute(
        &self,
        job: &Job,
        cancellation: Arc<AtomicBool>,
    ) -> Result<ExecutionOutcome, String> {
        match &job.kind {
            JobKind::AudioConversion {
                input_path,
                output_path,
                format,
            } => {
                let ffmpeg_path = self
                    .ffmpeg_path
                    .as_deref()
                    .ok_or("The bundled FFmpeg executable is unavailable")?;
                audio_converter::convert(
                    AudioConversion {
                        ffmpeg_path,
                        ffprobe_path: self.ffprobe_path.as_deref(),
                        input_path: Path::new(input_path),
                        output_path: Path::new(output_path),
                        format: *format,
                        job_id: &job.id,
                    },
                    cancellation,
                    |progress| self.update_progress(&job.id, progress),
                )
            }
            JobKind::PdfOperation { operation } => {
                let qpdf_path = self
                    .qpdf_path
                    .as_deref()
                    .ok_or("The bundled qpdf executable is unavailable")?;
                pdf_converter::execute(qpdf_path, operation, &job.id, cancellation, |progress| {
                    self.update_progress(&job.id, progress)
                })
            }
            JobKind::ImagesToPdf {
                input_paths,
                output_path,
            } => {
                let qpdf_path = self
                    .qpdf_path
                    .as_deref()
                    .ok_or("The bundled qpdf executable is unavailable")?;
                image_converter::convert(
                    qpdf_path,
                    input_paths,
                    Path::new(output_path),
                    &job.id,
                    cancellation,
                    |progress| self.update_progress(&job.id, progress),
                )
            }
            JobKind::PdfToImages {
                input_path,
                output_directory,
                format,
                dpi,
            } => {
                let pdfium_path = self
                    .pdfium_path
                    .as_deref()
                    .ok_or("The bundled PDFium library is unavailable")?;
                pdf_image_converter::convert(
                    pdfium_path,
                    Path::new(input_path),
                    Path::new(output_directory),
                    *format,
                    *dpi,
                    &job.id,
                    cancellation,
                    |progress| self.update_progress(&job.id, progress),
                )
            }
            JobKind::OfficeToPdf {
                input_path,
                output_path,
            } => {
                let libreoffice_path = self
                    .resolved_libreoffice_path()
                    .ok_or("LibreOffice is not installed")?;
                let qpdf_path = self
                    .qpdf_path
                    .as_deref()
                    .ok_or("The bundled qpdf executable is unavailable")?;
                office_converter::convert(
                    &libreoffice_path,
                    qpdf_path,
                    Path::new(input_path),
                    Path::new(output_path),
                    &job.id,
                    cancellation,
                    |progress| self.update_progress(&job.id, progress),
                )
            }
            JobKind::PdfToDocx {
                input_path,
                output_path,
            } => {
                let (python_path, worker_path) = self
                    .resolved_pdf_to_docx_runtime()
                    .ok_or("Python, PyMuPDF, and pdf2docx are required")?;
                pdf_docx_converter::convert(
                    &python_path,
                    &worker_path,
                    Path::new(input_path),
                    Path::new(output_path),
                    &job.id,
                    cancellation,
                    |progress| self.update_progress(&job.id, progress),
                )
            }
        }
    }

    fn update_progress(&self, id: &str, progress: u8) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let Some(job) = state.jobs.iter_mut().find(|job| job.id == id) else {
            return;
        };
        if progress <= job.progress {
            return;
        }
        job.progress = progress;
        let _ = self.persist(&state);
    }

    fn persist(&self, state: &ServiceState) -> Result<(), String> {
        job_history::replace_all(&self.history_database, &state.jobs)
    }

    fn resolved_libreoffice_path(&self) -> Option<PathBuf> {
        self.dependency_service
            .as_ref()
            .and_then(|service| service.libreoffice_path())
            .or_else(|| self.libreoffice_path.clone())
    }

    fn resolved_pdf_to_docx_runtime(&self) -> Option<(PathBuf, PathBuf)> {
        if let Some(service) = &self.dependency_service {
            return service
                .pdf_to_docx_runtime()
                .map(|runtime| (runtime.python_path, runtime.worker_path));
        }
        Some((
            self.python_path.clone()?,
            self.pdf_to_docx_worker_path.clone()?,
        ))
    }
}

fn plan_audio_folder_conversion(
    input_directory: &Path,
    output_directory: &Path,
    format: AudioFormat,
) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    if !input_directory.is_absolute() || !output_directory.is_absolute() {
        return Err("Input and output folders must use absolute paths".into());
    }
    if !input_directory.is_dir() {
        return Err("The selected input folder does not exist".into());
    }
    if !output_directory.is_dir() {
        return Err("The selected output folder does not exist".into());
    }

    let mut inputs = fs::read_dir(input_directory)
        .map_err(|error| format!("The selected input folder could not be read: {error}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && audio_converter::is_supported_input(path))
        .collect::<Vec<_>>();
    inputs.sort_by_key(|path| path.to_string_lossy().to_ascii_lowercase());

    if inputs.is_empty() {
        return Err(
            "No supported audio files were found in the selected folder (MP3, WAV, FLAC, AAC, M4A, or OGG)"
                .into(),
        );
    }

    let mut reserved_outputs = HashSet::new();
    let mut conversions = Vec::with_capacity(inputs.len());
    for input_path in inputs {
        let stem = input_path
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| {
                format!(
                    "An audio filename could not be converted to text: {}",
                    input_path.display()
                )
            })?;
        let output_path = next_available_output(
            output_directory,
            stem,
            format.extension(),
            &mut reserved_outputs,
        );
        audio_converter::validate_request(&input_path, &output_path, format)?;
        conversions.push((input_path, output_path));
    }
    Ok(conversions)
}

fn plan_image_folder_to_pdf(
    input_directory: &Path,
    output_directory: &Path,
    mode: ImageFolderMode,
    created_at: u64,
) -> Result<Vec<Job>, String> {
    validate_folder_pair(input_directory, output_directory)?;
    let inputs = collect_folder_files(input_directory, image_converter::is_supported_input)?;
    if inputs.is_empty() {
        return Err(
            "No supported image files were found in the selected folder (JPG, PNG, WebP, or TIFF)"
                .into(),
        );
    }

    let mut reserved = HashSet::new();
    match mode {
        ImageFolderMode::Combine => {
            let stem = input_directory
                .file_name()
                .and_then(|value| value.to_str())
                .filter(|value| !value.is_empty())
                .unwrap_or("images");
            let output_path = next_available_output(
                output_directory,
                &format!("{stem}-images"),
                "pdf",
                &mut reserved,
            );
            let input_paths = inputs
                .into_iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            image_converter::validate_request(&input_paths, &output_path)?;
            let id = format!(
                "{created_at}-{}",
                NEXT_JOB_ID.fetch_add(1, Ordering::Relaxed)
            );
            Ok(vec![Job::queued_images_to_pdf(
                id,
                input_paths,
                output_path.to_string_lossy().into_owned(),
                created_at,
            )])
        }
        ImageFolderMode::Separate => inputs
            .into_iter()
            .map(|input_path| {
                let output_path = next_available_output(
                    output_directory,
                    file_stem(&input_path)?,
                    "pdf",
                    &mut reserved,
                );
                let input_paths = vec![input_path.to_string_lossy().into_owned()];
                image_converter::validate_request(&input_paths, &output_path)?;
                let id = format!(
                    "{created_at}-{}",
                    NEXT_JOB_ID.fetch_add(1, Ordering::Relaxed)
                );
                Ok(Job::queued_images_to_pdf(
                    id,
                    input_paths,
                    output_path.to_string_lossy().into_owned(),
                    created_at,
                ))
            })
            .collect(),
    }
}

fn validate_folder_pair(input_directory: &Path, output_directory: &Path) -> Result<(), String> {
    if !input_directory.is_absolute() || !output_directory.is_absolute() {
        return Err("Input and output folders must use absolute paths".into());
    }
    if !input_directory.is_dir() {
        return Err("The selected input folder does not exist".into());
    }
    if !output_directory.is_dir() {
        return Err("The selected output folder does not exist".into());
    }
    Ok(())
}

fn collect_folder_files<F>(directory: &Path, supported: F) -> Result<Vec<PathBuf>, String>
where
    F: Fn(&Path) -> bool,
{
    let mut inputs = fs::read_dir(directory)
        .map_err(|error| format!("The selected input folder could not be read: {error}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && supported(path))
        .collect::<Vec<_>>();
    inputs.sort_by_key(|path| path.to_string_lossy().to_ascii_lowercase());
    Ok(inputs)
}

fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| {
            extensions
                .iter()
                .any(|extension| value.eq_ignore_ascii_case(extension))
        })
}

fn file_stem(path: &Path) -> Result<&str, String> {
    path.file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            format!(
                "A filename could not be converted to text: {}",
                path.display()
            )
        })
}

fn next_available_output(
    output_directory: &Path,
    stem: &str,
    extension: &str,
    reserved_outputs: &mut HashSet<String>,
) -> PathBuf {
    let mut suffix = 1_u32;
    loop {
        let filename = if suffix == 1 {
            format!("{stem}.{extension}")
        } else {
            format!("{stem}-{suffix}.{extension}")
        };
        let candidate = output_directory.join(filename);
        let key = candidate.to_string_lossy().to_ascii_lowercase();
        if !candidate.exists() && reserved_outputs.insert(key) {
            return candidate;
        }
        suffix += 1;
    }
}

fn next_available_directory(
    output_directory: &Path,
    stem: &str,
    reserved_outputs: &mut HashSet<String>,
) -> PathBuf {
    let mut suffix = 1_u32;
    loop {
        let name = if suffix == 1 {
            stem.to_string()
        } else {
            format!("{stem}-{suffix}")
        };
        let candidate = output_directory.join(name);
        let key = candidate.to_string_lossy().to_ascii_lowercase();
        if !candidate.exists() && reserved_outputs.insert(key) {
            return candidate;
        }
        suffix += 1;
    }
}

fn is_history_entry(status: &JobStatus) -> bool {
    matches!(
        status,
        JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
    )
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform;
    use std::time::Duration;
    use tempfile::tempdir;

    #[test]
    fn history_survives_service_recreation() {
        let directory = tempdir().unwrap();
        let service = Arc::new(
            JobService::new(directory.path().to_path_buf(), EnginePaths::default()).unwrap(),
        );
        {
            let mut state = service.state.lock().unwrap();
            state.jobs.push(Job::queued_audio(
                "persisted",
                "input.wav",
                "output.mp3",
                AudioFormat::Mp3,
                1,
            ));
            service.persist(&state).unwrap();
        }

        let recreated =
            JobService::new(directory.path().to_path_buf(), EnginePaths::default()).unwrap();
        assert_eq!(recreated.list_jobs().unwrap()[0].id, "persisted");
        assert!(directory.path().join("jobs.sqlite3").is_file());
    }

    #[test]
    fn legacy_json_history_is_imported_once_into_sqlite() {
        let directory = tempdir().unwrap();
        let job = Job::queued_audio("legacy", "input.wav", "output.mp3", AudioFormat::Mp3, 1);
        fs::write(
            directory.path().join("jobs.json"),
            serde_json::to_vec(&serde_json::json!({
                "version": 1,
                "jobs": [job],
            }))
            .unwrap(),
        )
        .unwrap();

        let service =
            JobService::new(directory.path().to_path_buf(), EnginePaths::default()).unwrap();
        assert_eq!(service.list_jobs().unwrap()[0].id, "legacy");
        assert!(directory.path().join("jobs.sqlite3").is_file());
    }

    #[test]
    fn completed_history_entry_can_be_deleted_without_removing_output() {
        let directory = tempdir().unwrap();
        let output = directory.path().join("converted.mp3");
        fs::write(&output, b"converted output").unwrap();
        let service =
            JobService::new(directory.path().join("history"), EnginePaths::default()).unwrap();
        {
            let mut state = service.state.lock().unwrap();
            let mut job = Job::queued_audio(
                "completed",
                "input.wav",
                output.to_string_lossy(),
                AudioFormat::Mp3,
                1,
            );
            job.status = JobStatus::Completed;
            job.progress = 100;
            job.finished_at = Some(2);
            state.jobs.push(job);
            service.persist(&state).unwrap();
        }

        service.delete_history_entry("completed").unwrap();
        assert!(service.list_jobs().unwrap().is_empty());
        assert_eq!(fs::read(&output).unwrap(), b"converted output");
        let recreated =
            JobService::new(directory.path().join("history"), EnginePaths::default()).unwrap();
        assert!(recreated.list_jobs().unwrap().is_empty());
    }

    #[test]
    fn active_history_entries_cannot_be_deleted() {
        let directory = tempdir().unwrap();
        let service =
            JobService::new(directory.path().to_path_buf(), EnginePaths::default()).unwrap();
        {
            let mut state = service.state.lock().unwrap();
            state.jobs.push(Job::queued_audio(
                "active",
                "input.wav",
                "output.mp3",
                AudioFormat::Mp3,
                1,
            ));
            service.persist(&state).unwrap();
        }

        let error = service.delete_history_entry("active").unwrap_err();
        assert!(error.contains("cancel"));
        assert_eq!(service.list_jobs().unwrap().len(), 1);
    }

    #[test]
    fn clear_history_preserves_active_jobs() {
        let directory = tempdir().unwrap();
        let service =
            JobService::new(directory.path().to_path_buf(), EnginePaths::default()).unwrap();
        {
            let mut state = service.state.lock().unwrap();
            for (id, status) in [
                ("completed", JobStatus::Completed),
                ("failed", JobStatus::Failed),
                ("cancelled", JobStatus::Cancelled),
                ("active", JobStatus::Queued),
            ] {
                let mut job = Job::queued_audio(id, "input.wav", "output.mp3", AudioFormat::Mp3, 1);
                job.status = status;
                state.jobs.push(job);
            }
            service.persist(&state).unwrap();
        }

        assert_eq!(service.clear_history().unwrap(), 2);
        let jobs = service.list_jobs().unwrap();
        assert_eq!(jobs.len(), 2);
        assert!(jobs.iter().any(|job| job.id == "active"));
        assert!(jobs.iter().any(|job| job.id == "failed"));
    }

    #[test]
    fn section_cleanup_removes_only_the_requested_terminal_group() {
        let directory = tempdir().unwrap();
        let service =
            JobService::new(directory.path().to_path_buf(), EnginePaths::default()).unwrap();
        {
            let mut state = service.state.lock().unwrap();
            for (id, status) in [
                ("completed", JobStatus::Completed),
                ("failed", JobStatus::Failed),
                ("cancelled", JobStatus::Cancelled),
                ("active", JobStatus::Queued),
            ] {
                let mut job = Job::queued_audio(id, "input.wav", "output.mp3", AudioFormat::Mp3, 1);
                job.status = status;
                state.jobs.push(job);
            }
            service.persist(&state).unwrap();
        }

        assert_eq!(service.clear_history_section("failed").unwrap(), 2);
        let jobs = service.list_jobs().unwrap();
        assert!(jobs.iter().any(|job| job.id == "completed"));
        assert!(jobs.iter().any(|job| job.id == "active"));
        assert!(!jobs.iter().any(|job| job.id == "failed"));
        assert!(!jobs.iter().any(|job| job.id == "cancelled"));
    }

    #[test]
    fn enqueue_reports_a_missing_engine_before_mutating_history() {
        let directory = tempdir().unwrap();
        let service = Arc::new(
            JobService::new(directory.path().to_path_buf(), EnginePaths::default()).unwrap(),
        );
        let error = service
            .enqueue_audio("input.wav".into(), "output.mp3".into(), AudioFormat::Mp3)
            .unwrap_err();

        assert!(error.contains("unavailable"));
        assert!(service.list_jobs().unwrap().is_empty());
    }

    #[test]
    fn folder_audio_plan_skips_unsupported_files_and_avoids_name_collisions() {
        let directory = tempdir().unwrap();
        let input = directory.path().join("input");
        let output = directory.path().join("output");
        fs::create_dir_all(input.join("nested")).unwrap();
        fs::create_dir(&output).unwrap();
        fs::write(input.join("song.m4a"), b"audio").unwrap();
        fs::write(input.join("song.wav"), b"audio").unwrap();
        fs::write(input.join("notes.txt"), b"not audio").unwrap();
        fs::write(input.join("nested").join("hidden.ogg"), b"audio").unwrap();
        fs::write(output.join("song.mp3"), b"existing").unwrap();

        let conversions = plan_audio_folder_conversion(&input, &output, AudioFormat::Mp3).unwrap();
        let outputs = conversions
            .iter()
            .map(|(_, output)| output.file_name().unwrap().to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert_eq!(conversions.len(), 2);
        assert_eq!(outputs, ["song-2.mp3", "song-3.mp3"]);
    }

    #[test]
    fn folder_audio_plan_reports_when_no_supported_files_exist() {
        let directory = tempdir().unwrap();
        let input = directory.path().join("input");
        let output = directory.path().join("output");
        fs::create_dir(&input).unwrap();
        fs::create_dir(&output).unwrap();
        fs::write(input.join("notes.txt"), b"not audio").unwrap();

        let error = plan_audio_folder_conversion(&input, &output, AudioFormat::Ogg).unwrap_err();
        assert!(error.contains("No supported audio files"));
    }

    #[test]
    fn folder_batch_helpers_skip_subfolders_and_allocate_safe_names() {
        let directory = tempdir().unwrap();
        let input = directory.path().join("input");
        let output = directory.path().join("output");
        fs::create_dir_all(input.join("nested")).unwrap();
        fs::create_dir(&output).unwrap();
        fs::write(input.join("photo.png"), b"image").unwrap();
        fs::write(input.join("report.docx"), b"office").unwrap();
        fs::write(input.join("document.pdf"), b"pdf").unwrap();
        fs::write(input.join("notes.txt"), b"unsupported").unwrap();
        fs::write(input.join("nested").join("hidden.pdf"), b"pdf").unwrap();

        let images = collect_folder_files(&input, image_converter::is_supported_input).unwrap();
        let office = collect_folder_files(&input, office_converter::is_supported_input).unwrap();
        let pdfs = collect_folder_files(&input, |path| has_extension(path, &["pdf"])).unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(office.len(), 1);
        assert_eq!(pdfs.len(), 1);

        fs::write(output.join("document.docx"), b"existing").unwrap();
        fs::create_dir(output.join("document")).unwrap();
        let mut reserved = HashSet::new();
        assert_eq!(
            next_available_output(&output, "document", "docx", &mut reserved),
            output.join("document-2.docx")
        );
        assert_eq!(
            next_available_directory(&output, "document", &mut reserved),
            output.join("document-2")
        );
    }

    #[test]
    fn image_folder_plan_supports_combined_and_separate_outputs() {
        let directory = tempdir().unwrap();
        let input = directory.path().join("album");
        let output = directory.path().join("output");
        fs::create_dir(&input).unwrap();
        fs::create_dir(&output).unwrap();
        fs::write(input.join("cover.jpg"), b"image").unwrap();
        fs::write(input.join("cover.png"), b"image").unwrap();
        fs::write(output.join("cover.pdf"), b"existing").unwrap();

        let combined =
            plan_image_folder_to_pdf(&input, &output, ImageFolderMode::Combine, 1).unwrap();
        assert_eq!(combined.len(), 1);
        let JobKind::ImagesToPdf {
            input_paths,
            output_path,
        } = &combined[0].kind
        else {
            panic!("expected an images-to-PDF job");
        };
        assert_eq!(input_paths.len(), 2);
        assert!(output_path.ends_with("album-images.pdf"));

        let separate =
            plan_image_folder_to_pdf(&input, &output, ImageFolderMode::Separate, 1).unwrap();
        let output_names = separate
            .iter()
            .map(|job| {
                let JobKind::ImagesToPdf {
                    input_paths,
                    output_path,
                } = &job.kind
                else {
                    panic!("expected an images-to-PDF job");
                };
                assert_eq!(input_paths.len(), 1);
                Path::new(output_path)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect::<Vec<_>>();
        assert_eq!(output_names, ["cover-2.pdf", "cover-3.pdf"]);
    }

    #[test]
    #[ignore = "runs the bundled FFmpeg binary and worker thread"]
    fn real_worker_completes_and_persists_an_audio_job() {
        let directory = tempdir().unwrap();
        let ffmpeg = platform::find_ffmpeg(None).expect("FFmpeg is required for this smoke test");
        let input = directory.path().join("worker-source.wav");
        let output = directory.path().join("worker-output.mp3");
        let generated = platform::background_command(&ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i"])
            .arg("sine=frequency=660:duration=0.25")
            .args(["-c:a", "pcm_s16le"])
            .arg(&input)
            .status()
            .unwrap();
        assert!(generated.success());

        let service = Arc::new(
            JobService::new(
                directory.path().join("history"),
                EnginePaths {
                    ffmpeg_path: Some(ffmpeg),
                    ffprobe_path: platform::find_ffprobe(None),
                    ..EnginePaths::default()
                },
            )
            .unwrap(),
        );
        let job = service
            .enqueue_audio(
                input.to_string_lossy().into_owned(),
                output.to_string_lossy().into_owned(),
                AudioFormat::Mp3,
            )
            .unwrap();

        let completed = (0..100).any(|_| {
            let current = service
                .list_jobs()
                .unwrap()
                .into_iter()
                .find(|item| item.id == job.id)
                .unwrap();
            if current.status == JobStatus::Completed {
                true
            } else {
                thread::sleep(Duration::from_millis(50));
                false
            }
        });

        assert!(completed, "worker did not finish within five seconds");
        assert!(fs::metadata(&output).unwrap().len() > 0);
        let recreated =
            JobService::new(directory.path().join("history"), EnginePaths::default()).unwrap();
        assert_eq!(
            recreated.list_jobs().unwrap()[0].status,
            JobStatus::Completed
        );
    }

    #[test]
    #[ignore = "runs the bundled FFmpeg binary and worker thread"]
    fn real_folder_batch_converts_every_audio_file_sequentially() {
        let directory = tempdir().unwrap();
        let input = directory.path().join("input");
        let output = directory.path().join("output");
        fs::create_dir(&input).unwrap();
        fs::create_dir(&output).unwrap();
        let ffmpeg = platform::find_ffmpeg(None).expect("FFmpeg is required for this smoke test");

        for (name, frequency) in [("first.wav", "440"), ("second.wav", "660")] {
            let generated = platform::background_command(&ffmpeg)
                .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i"])
                .arg(format!("sine=frequency={frequency}:duration=0.15"))
                .args(["-c:a", "pcm_s16le"])
                .arg(input.join(name))
                .status()
                .unwrap();
            assert!(generated.success());
        }

        let service = Arc::new(
            JobService::new(
                directory.path().join("history"),
                EnginePaths {
                    ffmpeg_path: Some(ffmpeg),
                    ffprobe_path: platform::find_ffprobe(None),
                    ..EnginePaths::default()
                },
            )
            .unwrap(),
        );
        let jobs = service
            .enqueue_audio_folder(
                input.to_string_lossy().into_owned(),
                output.to_string_lossy().into_owned(),
                AudioFormat::Mp3,
            )
            .unwrap();
        assert_eq!(jobs.len(), 2);

        let completed = (0..200).any(|_| {
            let current = service.list_jobs().unwrap();
            if current.len() == 2 && current.iter().all(|job| job.status == JobStatus::Completed) {
                true
            } else {
                thread::sleep(Duration::from_millis(50));
                false
            }
        });

        assert!(completed, "folder batch did not finish within ten seconds");
        assert!(fs::metadata(output.join("first.mp3")).unwrap().len() > 0);
        assert!(fs::metadata(output.join("second.mp3")).unwrap().len() > 0);
    }
}

mod conversion_tools;
mod job_tools;
mod paths;

use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use rmcp::{
    handler::server::router::tool::ToolRouter,
    model::{
        CallToolResult, Implementation, ProgressNotificationParam, RequestMetaObject,
        ServerCapabilities, ServerConfig,
    },
    schemars, tool_handler,
    transport::stdio,
    Peer, RoleServer, ServerHandler, ServiceExt,
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    application::{
        dependency_service::DependencyService,
        job_service::{EnginePaths, JobService},
    },
    domain::job::{Job, JobStatus},
    platform,
};

const APP_IDENTIFIER: &str = "com.erkan.justconvert";
const PRODUCT_NAME: &str = "Just Convert";
const DEFAULT_WAIT_SECONDS: u64 = 600;
const MAX_WAIT_SECONDS: u64 = 3600;
const POLL_INTERVAL: Duration = Duration::from_millis(250);

const INSTRUCTIONS: &str = "Just Convert converts local files entirely on this machine; nothing is uploaded. \
All paths must be absolute (a leading ~ is expanded). Originals are never modified and existing files are never overwritten. \
Output paths are optional: when omitted, the result is written next to the input with a non-colliding name. \
Conversion tools wait for completion by default and return the finished job including its output path; \
pass wait=false to return immediately and poll with get_job. Call list_converters to see which engines are available.";

#[derive(Clone)]
pub struct JustConvertServer {
    jobs: Arc<JobService>,
    dependencies: Arc<DependencyService>,
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
pub struct WaitOptions {
    /// Wait for the conversion to finish before returning. Defaults to true.
    pub wait: Option<bool>,
    /// Maximum seconds to wait (default 600, max 3600). The job keeps running after a timeout; poll it with get_job.
    pub timeout_seconds: Option<u64>,
}

impl JustConvertServer {
    pub fn new(jobs: Arc<JobService>, dependencies: Arc<DependencyService>) -> Self {
        Self {
            jobs,
            dependencies,
            tool_router: Self::conversion_tools() + Self::job_tools(),
        }
    }

    async fn finish(
        &self,
        result: Result<Vec<Job>, String>,
        options: WaitOptions,
        meta: RequestMetaObject,
        peer: Peer<RoleServer>,
    ) -> CallToolResult {
        let jobs = match result {
            Ok(jobs) => jobs,
            Err(message) => return tool_error(message),
        };
        if jobs.is_empty() {
            return tool_error("No supported input files were found");
        }
        if options.wait == Some(false) {
            return job_result(jobs, false);
        }

        let ids: Vec<String> = jobs.iter().map(|job| job.id.clone()).collect();
        let timeout = Duration::from_secs(
            options
                .timeout_seconds
                .unwrap_or(DEFAULT_WAIT_SECONDS)
                .min(MAX_WAIT_SECONDS),
        );
        let progress_token = meta.get_progress_token();
        let started = Instant::now();
        let mut last_progress = None;

        loop {
            let current = match self.jobs.list_jobs() {
                Ok(all) => all
                    .into_iter()
                    .filter(|job| ids.contains(&job.id))
                    .collect::<Vec<_>>(),
                Err(message) => return tool_error(message),
            };
            let done = current.iter().all(|job| is_terminal(&job.status));
            if done || started.elapsed() >= timeout {
                return job_result(current, done);
            }

            if let Some(token) = &progress_token {
                let progress = current
                    .iter()
                    .map(|job| f64::from(job.progress))
                    .sum::<f64>()
                    / ids.len() as f64;
                if last_progress != Some(progress) {
                    last_progress = Some(progress);
                    let _ = peer
                        .notify_progress(
                            ProgressNotificationParam::new(token.clone(), progress)
                                .with_total(100.0),
                        )
                        .await;
                }
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for JustConvertServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(
                Implementation::new("just-convert", env!("CARGO_PKG_VERSION"))
                    .with_title("Just Convert"),
            )
            .with_instructions(INSTRUCTIONS)
    }
}

fn is_terminal(status: &JobStatus) -> bool {
    matches!(
        status,
        JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
    )
}

fn job_result(jobs: Vec<Job>, finished: bool) -> CallToolResult {
    let failed = finished
        && jobs
            .iter()
            .all(|job| matches!(job.status, JobStatus::Failed | JobStatus::Cancelled));
    let mut value = json!({ "finished": finished, "jobs": jobs });
    if !finished {
        value["hint"] = json!("Still running. Poll with get_job using the job id.");
    }
    if failed {
        CallToolResult::structured_error(value)
    } else {
        CallToolResult::structured(value)
    }
}

fn tool_error(message: impl Into<String>) -> CallToolResult {
    CallToolResult::structured_error(json!({ "error": message.into() }))
}

/// Locates the app's bundled resources relative to the running executable: next to it on
/// Windows, in `Contents/Resources` inside a macOS app, and in `lib/<product>` on Linux.
fn installed_resource_directory() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    let directory = executable.parent()?;
    [
        directory.to_path_buf(),
        directory.join("../Resources"),
        directory.join("../lib").join(PRODUCT_NAME),
    ]
    .into_iter()
    .find(|candidate| candidate.join("workers/pdf_to_docx.py").is_file())
    .or_else(|| Some(directory.to_path_buf()))
}

/// Runs the MCP server over stdio until the client disconnects.
pub fn serve() -> Result<(), String> {
    let gui_data_directory = match std::env::var_os("JUST_CONVERT_DATA_DIR") {
        Some(directory) => PathBuf::from(directory),
        None => dirs::data_dir()
            .ok_or("The application data directory could not be determined")?
            .join(APP_IDENTIFIER),
    };
    let resource_directory = installed_resource_directory();

    // Engine overrides are shared with the desktop app; job history is kept separate so both can run at once.
    let dependencies = Arc::new(DependencyService::new(
        &gui_data_directory,
        resource_directory.clone(),
    )?);
    let jobs = Arc::new(JobService::new(
        gui_data_directory.join("mcp"),
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
    jobs.start();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("Async runtime could not be started: {error}"))?;
    runtime.block_on(async {
        let service = JustConvertServer::new(jobs, dependencies)
            .serve(stdio())
            .await
            .map_err(|error| format!("MCP session could not be started: {error}"))?;
        service
            .waiting()
            .await
            .map_err(|error| format!("MCP session ended unexpectedly: {error}"))?;
        Ok(())
    })
}

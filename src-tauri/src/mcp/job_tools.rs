use rmcp::{
    handler::server::wrapper::Parameters,
    model::{CallToolResult, RequestMetaObject},
    schemars, tool, tool_router, ErrorData, Peer, RoleServer,
};
use serde::Deserialize;
use serde_json::json;

use super::{tool_error, JustConvertServer, WaitOptions};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListJobsRequest {
    /// Maximum number of jobs to return, newest first. Defaults to 20.
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetJobRequest {
    /// Job id returned by a conversion tool.
    pub id: String,
    #[serde(flatten)]
    pub wait: WaitOptions,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct JobIdRequest {
    /// Job id returned by a conversion tool.
    pub id: String,
}

const DEFAULT_JOB_LIMIT: usize = 20;

#[tool_router(router = job_tools, vis = "pub(super)")]
impl JustConvertServer {
    #[tool(
        description = "List recent conversion jobs started through this server, newest first.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_jobs(
        &self,
        Parameters(request): Parameters<ListJobsRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(match self.jobs.list_jobs() {
            Ok(mut jobs) => {
                jobs.truncate(request.limit.unwrap_or(DEFAULT_JOB_LIMIT));
                CallToolResult::structured(json!({ "jobs": jobs }))
            }
            Err(message) => tool_error(message),
        })
    }

    #[tool(
        description = "Get a job's status and output. Waits for it to finish unless wait=false.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn get_job(
        &self,
        Parameters(request): Parameters<GetJobRequest>,
        meta: RequestMetaObject,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = self.jobs.list_jobs().and_then(|jobs| {
            jobs.into_iter()
                .find(|job| job.id == request.id)
                .map(|job| vec![job])
                .ok_or_else(|| format!("No job with id {}", request.id))
        });
        Ok(self.finish(result, request.wait, meta, peer).await)
    }

    #[tool(
        description = "Cancel a queued or running job. Partial output is removed.",
        annotations(
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn cancel_job(
        &self,
        Parameters(request): Parameters<JobIdRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(match self.jobs.cancel(&request.id) {
            Ok(job) => CallToolResult::structured(json!({ "job": job })),
            Err(message) => tool_error(message),
        })
    }
}

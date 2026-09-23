use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use rusqlite::{params, Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};

use crate::domain::job::Job;

const DATABASE_NAME: &str = "jobs.sqlite3";
const LEGACY_JSON_NAME: &str = "jobs.json";

#[derive(Deserialize, Serialize)]
struct PersistedJobs {
    version: u8,
    jobs: Vec<Job>,
}

pub fn initialize(data_directory: &Path) -> Result<(PathBuf, Vec<Job>), String> {
    let database_path = data_directory.join(DATABASE_NAME);
    let legacy_path = data_directory.join(LEGACY_JSON_NAME);
    let database_existed = database_path.exists();

    let connection = open(&database_path)?;
    initialize_schema(&connection)?;
    let mut jobs = load_from_database(&connection)?;

    if !database_existed && jobs.is_empty() && legacy_path.is_file() {
        jobs = load_legacy_json(&legacy_path)?;
        replace_all_with_connection(&mut connection_for_write(&database_path)?, &jobs)?;
    }

    Ok((database_path, jobs))
}

pub fn replace_all(database_path: &Path, jobs: &[Job]) -> Result<(), String> {
    let mut connection = connection_for_write(database_path)?;
    replace_all_with_connection(&mut connection, jobs)
}

fn connection_for_write(path: &Path) -> Result<Connection, String> {
    let connection = open(path)?;
    initialize_schema(&connection)?;
    Ok(connection)
}

fn open(path: &Path) -> Result<Connection, String> {
    let connection = Connection::open(path)
        .map_err(|error| format!("Job history database could not be opened: {error}"))?;
    connection
        .busy_timeout(Duration::from_secs(3))
        .map_err(|error| {
            format!("Job history database timeout could not be configured: {error}")
        })?;
    Ok(connection)
}

fn initialize_schema(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS job_history (
               id TEXT PRIMARY KEY NOT NULL,
               created_at INTEGER NOT NULL,
               payload TEXT NOT NULL
             );",
        )
        .map_err(|error| format!("Job history database could not be initialized: {error}"))
}

fn load_from_database(connection: &Connection) -> Result<Vec<Job>, String> {
    let mut statement = connection
        .prepare("SELECT payload FROM job_history ORDER BY created_at ASC, id ASC")
        .map_err(|error| format!("Job history query could not be prepared: {error}"))?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| format!("Job history could not be queried: {error}"))?;

    let mut jobs = Vec::new();
    for row in rows {
        let payload = row.map_err(|error| format!("Job history row could not be read: {error}"))?;
        jobs.push(
            serde_json::from_str(&payload)
                .map_err(|error| format!("Job history entry is invalid: {error}"))?,
        );
    }
    Ok(jobs)
}

fn replace_all_with_connection(connection: &mut Connection, jobs: &[Job]) -> Result<(), String> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| format!("Job history transaction could not be started: {error}"))?;
    transaction
        .execute("DELETE FROM job_history", [])
        .map_err(|error| format!("Old job history could not be cleared: {error}"))?;
    {
        let mut insert = transaction
            .prepare("INSERT INTO job_history (id, created_at, payload) VALUES (?1, ?2, ?3)")
            .map_err(|error| format!("Job history insert could not be prepared: {error}"))?;
        for job in jobs {
            let payload = serde_json::to_string(job)
                .map_err(|error| format!("Job history could not be serialized: {error}"))?;
            let created_at = i64::try_from(job.created_at)
                .map_err(|_| "Job timestamp is too large for SQLite".to_string())?;
            insert
                .execute(params![job.id, created_at, payload])
                .map_err(|error| format!("Job history entry could not be written: {error}"))?;
        }
    }
    transaction
        .commit()
        .map_err(|error| format!("Job history transaction could not be committed: {error}"))
}

fn load_legacy_json(path: &Path) -> Result<Vec<Job>, String> {
    let payload =
        fs::read(path).map_err(|error| format!("Legacy job history could not be read: {error}"))?;
    let persisted: PersistedJobs = serde_json::from_slice(&payload)
        .map_err(|error| format!("Legacy job history is invalid: {error}"))?;
    if persisted.version != 1 {
        return Err(format!(
            "Unsupported legacy job history version: {}",
            persisted.version
        ));
    }
    Ok(persisted.jobs)
}

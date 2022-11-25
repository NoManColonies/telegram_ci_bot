use super::bot::state::DeployStatus;
use crate::app::{
    middleware::auth::service::SessionContainer,
    util::{empty_string_deserializer::empty_string_as_none, error::ServiceError},
};
use axum::{response::IntoResponse, Extension, Json};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use http::StatusCode;
use serde::Deserialize;
use sqlx::{query, Pool, Sqlite};
use teloxide::{requests::Requester, types::ChatId, utils::markdown::link, Bot};
use tracing::info;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobCreationBody {
    job_id: i32,
    #[serde(deserialize_with = "empty_string_as_none")]
    url: Option<String>,
    #[serde(deserialize_with = "empty_string_as_none")]
    description: Option<String>,
    #[serde(deserialize_with = "empty_string_as_none")]
    by: Option<String>,
    #[serde(deserialize_with = "empty_string_as_none")]
    by_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobStatusBody {
    job_id: i32,
    status: DeployStatus,
    #[serde(deserialize_with = "empty_string_as_none")]
    description: Option<String>,
    #[serde(deserialize_with = "empty_string_as_none")]
    by: Option<String>,
}

fn format_create_message(
    repo_name: String,
    url: Option<String>,
    description: Option<String>,
    by: Option<String>,
    by_name: Option<String>,
) -> String {
    let mut text = description.map_or(format!("🚧 {repo_name}'s job is running..."), |dsc| dsc);

    if let (Some(by), Some(by_name)) = (by, by_name) {
        text = format!("{text}\nby: {}", link(&by, &by_name));
    }

    if let Some(url) = url {
        text = format!("{text}\nlink: {}", link(&url, &repo_name));
    }

    text
}

fn format_update_message(
    repo_name: String,
    status: DeployStatus,
    elapsed: String,
    url: Option<String>,
    description: Option<String>,
    by: Option<String>,
    by_name: Option<String>,
) -> Result<String, ServiceError> {
    let mut text = description.map_or(
        match status {
            DeployStatus::Success => {
                format!("✅ {repo_name}'s job has completed")
            }
            DeployStatus::Failure => {
                format!("🚨 {repo_name}'s job encountered failure")
            }
            DeployStatus::Cancelled => {
                format!("⛔️ {repo_name}'s job was cancelled")
            }
            _ => {
                return Err(ServiceError::ParseMessage(format!(
                    "Invalid job status: {status}"
                )))
            }
        },
        |dsc| dsc,
    );
    text = format!("{text}\nelapsed: {elapsed}");

    if let (Some(by), Some(by_name)) = (by, by_name) {
        text = format!("{text}\nby: {}", link(&by, &by_name));
    }

    if let Some(url) = url {
        text = format!("{text}\nlink: {}", link(&url, &repo_name));
    }

    Ok(text)
}

pub async fn create_job_handler(
    Extension(SessionContainer(session)): Extension<SessionContainer>,
    Extension(pool): Extension<Pool<Sqlite>>,
    Extension(bot): Extension<Bot>,
    Json(JobCreationBody {
        job_id,
        url,
        description,
        by,
        by_name,
    }): Json<JobCreationBody>,
) -> impl IntoResponse {
    if let Some(session) = session {
        let record = query!(
            r#"
            SELECT message_id, name
            FROM main.repos
            WHERE id = ?
            "#,
            session.sid
        )
        .fetch_one(&pool)
        .await?;
        query!(
            r#"
            INSERT INTO main.jobs
            (id, status, triggered_by, description, callback_url, repo_id)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
            job_id,
            DeployStatus::Running,
            by_name,
            description,
            url,
            session.sid
        )
        .execute(&pool)
        .await?;

        bot.send_message(
            ChatId(record.message_id),
            format_create_message(record.name, url, description, by, by_name),
        )
        .await?;

        Ok(StatusCode::OK)
    } else {
        Err(ServiceError::BadCredential)
    }
}

fn format_duration(elapsed: Duration) -> String {
    if elapsed.num_seconds() < 60 {
        return format!("{} second(s)", elapsed.num_seconds());
    }
    if elapsed.num_minutes() < 60 {
        return format!("{} minute(s)", elapsed.num_minutes());
    }
    if elapsed.num_hours() < 24 {
        return format!("{} hour(s)", elapsed.num_hours());
    }
    format!("{} day(s)", elapsed.num_days())
}

pub async fn update_job_handler(
    Extension(SessionContainer(session)): Extension<SessionContainer>,
    Extension(pool): Extension<Pool<Sqlite>>,
    Extension(bot): Extension<Bot>,
    Json(JobStatusBody {
        job_id,
        status,
        description,
        by,
    }): Json<JobStatusBody>,
) -> impl IntoResponse {
    if let Some(session) = session {
        let mut transaction = pool.begin().await?;
        let record = query!(
            r#"
            SELECT repos.message_id, 
                repos.name, 
                jobs.callback_url, 
                jobs.triggered_by, 
                jobs.started_at
            FROM main.jobs
            JOIN repos ON jobs.repo_id = repos.id
            WHERE repos.id = ?
            AND jobs.status = ?
            "#,
            session.sid,
            DeployStatus::Running
        )
        .fetch_one(&mut transaction)
        .await?;
        let now = Utc::now().naive_utc();
        let elapsed = now - record.started_at;
        let elapsed_seconds = elapsed.num_seconds();
        query!(
            r#"
            UPDATE main.jobs
            SET status = ?,
                elapsed = ?
            WHERE id = ?
            AND repo_id = ?
            "#,
            status,
            elapsed_seconds,
            job_id,
            session.sid
        )
        .execute(&mut transaction)
        .await?;

        bot.send_message(
            ChatId(record.message_id),
            format_update_message(
                record.name,
                status,
                format_duration(elapsed),
                record.callback_url,
                description,
                by,
                record.triggered_by,
            )?,
        )
        .await?;
        transaction.commit().await?;

        Ok(StatusCode::OK)
    } else {
        Err(ServiceError::BadCredential)
    }
}

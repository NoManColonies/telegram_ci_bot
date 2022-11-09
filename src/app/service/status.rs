use super::bot::state::DeployStatus;
use crate::app::{middleware::auth::service::SessionContainer, util::error::ServiceError};
use axum::{extract::Query, response::IntoResponse, Extension};
use http::StatusCode;
use serde::Deserialize;
use sqlx::{query, Pool, Sqlite};
use teloxide::{requests::Requester, types::ChatId, Bot};
use tracing::info;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusQuery {
    status: DeployStatus,
    url: Option<String>,
    description: Option<String>,
}

const DEPLOYING_TEXT: &'static str = "is deploying... ⚙️";
const DEPLOY_SUCCESS_TEXT: &'static str = "deployed successfully 🎉";
const DEPLOY_FAILURE_TEXT: &'static str = "failed to deploy 🔥";

fn format_telegram_message(
    status: DeployStatus,
    repo_name: String,
    last_status: DeployStatus,
    url: Option<String>,
    description: Option<String>,
) -> String {
    match (status, last_status, description, url) {
        (DeployStatus::Idle, last_status, _, _) if last_status != DeployStatus::Deploy => {
            format!("repo: {repo_name} is doing nothing 💤")
        }
        (DeployStatus::Idle, _, Some(description), _) => description,
        (DeployStatus::Idle, _, _, _) => {
            format!("repo: {repo_name} deployment was cancelled ⛔️")
        }
        (DeployStatus::Deploy, _, Some(description), Some(url)) => {
            format!("{description}\n\nlink: {url}")
        }
        (DeployStatus::Deploy, _, Some(description), _) => description,
        (DeployStatus::Deploy, _, _, Some(url)) => {
            format!("repo: {repo_name} is deploying... ⚙️\n\nlink: {url}")
        }
        (DeployStatus::Deploy, _, _, _) => format!("repo: {repo_name} {DEPLOYING_TEXT}"),
        (DeployStatus::Success, _, Some(description), Some(url)) => {
            format!("{description}\n\nlink: {url}")
        }
        (DeployStatus::Success, _, Some(description), _) => description,
        (DeployStatus::Success, _, _, Some(url)) => {
            format!("repo: {repo_name} {DEPLOY_SUCCESS_TEXT}\n\nlink: {url}")
        }
        (DeployStatus::Success, _, _, _) => format!("repo: {repo_name} {DEPLOY_SUCCESS_TEXT}"),
        (DeployStatus::Failure, _, Some(description), Some(url)) => {
            format!("{description}\n\nlink: {url}")
        }
        (DeployStatus::Failure, _, Some(description), _) => description,
        (DeployStatus::Failure, _, _, Some(url)) => {
            format!("repo: {repo_name} {DEPLOY_FAILURE_TEXT}\n\nlink: {url}")
        }
        (DeployStatus::Failure, _, _, _) => format!("repo: {repo_name} {DEPLOY_FAILURE_TEXT}"),
    }
}

pub async fn update_status(
    Extension(SessionContainer(session)): Extension<SessionContainer>,
    Extension(pool): Extension<Pool<Sqlite>>,
    Extension(bot): Extension<Bot>,
    Query(StatusQuery {
        status,
        url,
        description,
    }): Query<StatusQuery>,
) -> impl IntoResponse {
    if let Some(session) = session {
        let record = query!(
            r#"
            SELECT message_id, name, status
            FROM main.repos
            WHERE id = ?
            "#,
            session.sid
        )
        .fetch_one(&pool)
        .await?;
        let result = query!(
            r#"
            UPDATE main.repos
            SET status = ?
            WHERE id = ?
            "#,
            status,
            session.sid
        )
        .execute(&pool)
        .await?;

        info!(
            "Query success with affacted rows: {}",
            result.rows_affected()
        );
        bot.send_message(
            ChatId(record.message_id),
            format_telegram_message(
                status,
                record.name,
                DeployStatus::try_from(record.status.as_str())?,
                url,
                description,
            ),
        )
        .await?;

        Ok(StatusCode::OK)
    } else {
        Err(ServiceError::BadCredential)
    }
}

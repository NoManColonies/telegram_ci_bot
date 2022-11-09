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

fn format_telegram_message(
    status: DeployStatus,
    repo_name: String,
    last_status: DeployStatus,
    url: Option<String>,
    description: Option<String>,
) -> String {
    match (status, last_status, description, url) {
        (DeployStatus::Idle, last_status, _, _) if last_status != DeployStatus::Deploy => {
            format!("repo: {} is doing nothing 💤", repo_name)
        }
        (DeployStatus::Idle, _, Some(description), _) => description,
        (DeployStatus::Idle, _, _, _) => {
            format!("repo: {} deployment was cancelled ⛔️", repo_name)
        }
        (DeployStatus::Deploy, _, Some(description), _) => description,
        (DeployStatus::Deploy, _, _, _) => format!("repo: {} is deploying... ⚙️", repo_name),
        (DeployStatus::Success, _, Some(description), _) => description,
        (DeployStatus::Success, _, _, _) => format!("repo: {} deployed successfully 🎉", repo_name),
        (DeployStatus::Failure, _, Some(description), _) => description,
        (DeployStatus::Failure, _, _, _) => format!("repo: {} failed to deploy 🔥", repo_name),
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

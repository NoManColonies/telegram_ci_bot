use super::bot::state::DeployStatus;
use crate::app::{middleware::auth::service::SessionContainer, util::error::ServiceError};
use axum::{extract::Query, response::IntoResponse, Extension};
use http::StatusCode;
use serde::Deserialize;
use sqlx::{query, Pool, Sqlite};
use teloxide::{requests::Requester, types::ChatId, Bot};
use tracing::info;

#[derive(Deserialize)]
pub struct StatusQuery {
    status: DeployStatus,
}

pub async fn update_status(
    Extension(SessionContainer(session)): Extension<SessionContainer>,
    Extension(pool): Extension<Pool<Sqlite>>,
    Extension(bot): Extension<Bot>,
    Query(StatusQuery { status }): Query<StatusQuery>,
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
            match status {
                DeployStatus::Idle => format!("repo: {} deployment was cancelled ⛔️", record.name),
                DeployStatus::Deploy => format!("repo: {} is deploying... ⚙️", record.name),
                DeployStatus::Success => format!("repo: {} deployed successfully 🎉", record.name),
                DeployStatus::Failure => format!("repo: {} failed to deploy 🔥", record.name),
            },
        )
        .await?;

        Ok(StatusCode::OK)
    } else {
        Err(ServiceError::BadCredential)
    }
}

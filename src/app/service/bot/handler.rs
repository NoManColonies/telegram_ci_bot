use super::state::{BotState, DeployStatus, GeneralCommand, RepoCommand};
use crate::app::util::error::ServiceError;
use chrono::prelude::*;
use sqlx::{query, query_as, sqlite::SqliteRow, Pool, Row, Sqlite};
use teloxide::{
    dispatching::dialogue::ErasedStorage, prelude::*, types::MenuButton,
    utils::command::BotCommands,
};
use tracing::info;
use uuid::Uuid;

pub type MyDialogue = Dialogue<BotState, ErasedStorage<BotState>>;
pub type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

pub async fn start(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(
        msg.chat.id,
        "Let's start by configuring your first repo. Type /help for more info",
    )
    .await?;
    bot.set_chat_menu_button()
        .menu_button(MenuButton::Commands)
        .chat_id(msg.chat.id)
        .await?;
    dialogue.update(BotState::NormalMode(vec![])).await?;
    Ok(())
}

pub async fn config_mode_handler(
    bot: Bot,
    dialogue: MyDialogue,
    (repos, repo_key): (Vec<String>, String),
    sqlite_pool: Pool<Sqlite>,
    msg: Message,
    cmd: RepoCommand,
) -> HandlerResult {
    info!("received message: {}", msg.chat.id);
    match cmd {
        RepoCommand::Help => {
            bot.send_message(msg.chat.id, RepoCommand::descriptions().to_string())
                .await?;
        }
        RepoCommand::Today => {
            let date_time = Utc::now().naive_utc();
            let Some(beginning_of_today) = date_time.with_hour(0) else { return Err(Box::new(ServiceError::ChronoDatetime)); };
            let records = query!(
                r#"
                SELECT *
                FROM main.jobs
                WHERE repo_id = ?
                AND started_at >= ?
                "#,
                repo_key,
                beginning_of_today
            )
            .fetch_all(&sqlite_pool)
            .await?;

            if records.is_empty() {
                bot.send_message(
                    msg.chat.id,
                    "No running job today. Start running job to see them here.",
                )
                .await?;
            } else {
                bot.send_message(msg.chat.id, format!("{:?}", records))
                    .await?;
            }
        }
        RepoCommand::Latest => {
            let record = query!(
                r#"
                SELECT *
                FROM main.jobs
                WHERE repo_id = ?
                ORDER BY started_at DESC
                "#,
                repo_key,
            )
            .fetch_optional(&sqlite_pool)
            .await?;

            if record.is_none() {
                bot.send_message(
                    msg.chat.id,
                    "No latest running job. Start running job to see them here.",
                )
                .await?;
            } else {
                bot.send_message(msg.chat.id, format!("{:?}", record))
                    .await?;
            }
        }
        RepoCommand::Delete => {
            let mut transaction = sqlite_pool.begin().await?;
            let result = query!(
                r#"
                DELETE FROM main.repos
                WHERE id = ?
                "#,
                repo_key
            )
            .execute(&mut transaction)
            .await?;
            info!(
                "Query success with affacted rows: {}",
                result.rows_affected()
            );
            bot.send_message(msg.chat.id, format!("Successfully deleted repo."))
                .await?;
            let repos = repos.into_iter().filter(|repo| repo == &repo_key).collect();
            dialogue.update(BotState::NormalMode(repos)).await?;
            transaction.commit().await?;
        }
        RepoCommand::Cancel => {
            dialogue.update(BotState::NormalMode(repos)).await?;
            bot.set_chat_menu_button()
                .menu_button(MenuButton::Commands)
                .chat_id(msg.chat.id)
                .await?;
        }
        RepoCommand::GetInfo => {
            let record = query!(
                r#"
                SELECT id, name 
                FROM main.repos
                WHERE id = ?
                "#,
                repo_key
            )
            .fetch_one(&sqlite_pool)
            .await?;
            bot.send_message(msg.chat.id, format!("name: {}", record.name))
                .await?;
            bot.send_message(msg.chat.id, format!("key: ||{}||", record.id))
                .await?;
        }
        RepoCommand::Running => {
            let records = query!(
                r#"
                SELECT *
                FROM main.jobs
                WHERE repo_id = ?
                AND status = ?
                "#,
                repo_key,
                DeployStatus::Running
            )
            .fetch_all(&sqlite_pool)
            .await?;

            if records.is_empty() {
                bot.send_message(
                    msg.chat.id,
                    "No running job configured. Start running job to see them here.",
                )
                .await?;
            } else {
                bot.send_message(msg.chat.id, format!("{:?}", records))
                    .await?;
            }
        }
        RepoCommand::Rename(new_name) => {
            query!(
                r#"
                UPDATE main.repos
                SET name = ?
                WHERE id = ?
                "#,
                new_name,
                repo_key
            )
            .execute(&sqlite_pool)
            .await?;
            bot.send_message(msg.chat.id, "Successfully updated repo name")
                .await?;
        }
    };
    Ok(())
}

#[derive(sqlx::FromRow, Debug)]
struct JobProp<T> {
    id: i32,
    status: DeployStatus,
    triggered_by: Option<String>,
    description: Option<String>,
    callback_url: Option<String>,
    repo_id: T,
    started_at: DateTime<Utc>,
    elapsed: i32,
}

pub async fn normal_mode_handler(
    bot: Bot,
    dialogue: MyDialogue,
    mut repos: Vec<String>,
    sqlite_pool: Pool<Sqlite>,
    msg: Message,
    cmd: GeneralCommand,
) -> HandlerResult {
    match cmd {
        GeneralCommand::Help => {
            bot.send_message(msg.chat.id, GeneralCommand::descriptions().to_string())
                .await?;
        }
        GeneralCommand::List => {
            let records: Vec<(String, String)> = query(&format!(
                r#"
                SELECT id, name
                FROM main.repos
                WHERE id IN ({})
                "#,
                repos
                    .into_iter()
                    .map(|repo| format!("\"{repo}\""))
                    .collect::<Vec<String>>()
                    .join(",")
            ))
            .map(|row: SqliteRow| (row.get(0), row.get(1)))
            .fetch_all(&sqlite_pool)
            .await?;

            if records.is_empty() {
                bot.send_message(
                    msg.chat.id,
                    "No repo configured. Type /help to get started.",
                )
                .await?;
            } else {
                bot.send_message(msg.chat.id, format!("{:?}", records))
                    .await?;
            }
        }
        GeneralCommand::Today => {
            let date_time = Utc::now().naive_utc();
            let Some(beginning_of_today) = date_time.with_hour(0) else { return Err(Box::new(ServiceError::ChronoDatetime)); };
            let records = query_as::<_, JobProp<String>>(&format!(
                r#"
                SELECT *
                FROM main.jobs
                WHERE repo_id IN ({})
                AND started_at >= ?
                "#,
                repos
                    .into_iter()
                    .map(|repo| format!("\"{repo}\""))
                    .collect::<Vec<String>>()
                    .join(",")
            ))
            .bind(beginning_of_today)
            .fetch_all(&sqlite_pool)
            .await?;

            if records.is_empty() {
                bot.send_message(
                    msg.chat.id,
                    "No running job today. Start running job to see them here.",
                )
                .await?;
            } else {
                bot.send_message(msg.chat.id, format!("{:?}", records))
                    .await?;
            }
        }
        GeneralCommand::Create(name) => {
            let mut transaction = sqlite_pool.begin().await?;
            let uuid = Uuid::new_v4().simple().to_string();
            query!(
                r#"
                INSERT INTO main.repos 
                (id, name, message_id)
                VALUES (?, ?, ?)
                "#,
                uuid,
                name,
                msg.chat.id.0
            )
            .execute(&mut transaction)
            .await?;

            bot.send_message(msg.chat.id, format!("Successfully added repo: {name}"))
                .await?;
            bot.send_message(msg.chat.id, format!("key: {uuid}"))
                .await?;
            repos.push(uuid);
            dialogue.update(BotState::NormalMode(repos)).await?;
            transaction.commit().await?;
        }
        GeneralCommand::SelectRepo(index) => {
            match repos.get(index - 1) {
                Some(repo) => {
                    let record = query!(
                        r#"
                        SELECT id, name
                        FROM main.repos
                        WHERE id = ?
                        "#,
                        repo
                    )
                    .fetch_one(&sqlite_pool)
                    .await?;

                    bot.send_message(msg.chat.id, format!("Seleceing repo name: {}", record.name))
                        .await?;
                    dialogue
                        .update(BotState::ConfigMode(repos, record.id))
                        .await?;
                }
                None => {
                    bot.send_message(msg.chat.id, "Requested repo does not exists.")
                        .await?;
                }
            };
        }
        GeneralCommand::Reset => {
            query!(
                r#"
                DELETE FROM main.repos
                WHERE message_id = ?
                "#,
                msg.chat.id.0
            )
            .execute(&sqlite_pool)
            .await?;
            bot.send_message(msg.chat.id, "[DEBUG] Successfully reset all state")
                .await?;
            dialogue.update(BotState::Start).await?;
        }
    };
    Ok(())
}

pub async fn invalid_command(bot: Bot, msg: Message) -> HandlerResult {
    info!("invalid command: {}", msg.chat.id);
    bot.send_message(msg.chat.id, "Invalid command. see /help for more info.")
        .await?;
    Ok(())
}

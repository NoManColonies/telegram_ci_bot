use super::state::{BotState, DeployStatus, RepoCommand};
use sqlx::{query, Pool, Sqlite};
use teloxide::{dispatching::dialogue::ErasedStorage, prelude::*, utils::command::BotCommands};
use tracing::info;
use uuid::Uuid;

pub type MyDialogue = Dialogue<BotState, ErasedStorage<BotState>>;
pub type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

pub async fn start(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Let's start by configuring repo name")
        .await?;
    dialogue.update(BotState::ConfigMode).await?;
    Ok(())
}

pub async fn config_repo(
    bot: Bot,
    dialogue: MyDialogue,
    sqlite_pool: Pool<Sqlite>,
    msg: Message,
) -> HandlerResult {
    info!("received message: {}", msg.chat.id);
    if let Some(text) = msg.text().map(ToOwned::to_owned) {
        let uuid = Uuid::new_v4();
        let key = uuid.simple().to_string();
        let result = query!(
            r#"
            INSERT INTO main.repos 
            (id, name, status, message_id) VALUES
            (?, ?, ?, ?)
            "#,
            key,
            text,
            DeployStatus::Idle,
            msg.chat.id.0
        )
        .execute(&sqlite_pool)
        .await?;
        info!(
            "Query success with affected rows: {}",
            result.rows_affected()
        );
        bot.send_message(
            msg.chat.id,
            "Repo successfully configured. Listening for deployment status...",
        )
        .await?;
        bot.send_message(msg.chat.id, format!("key: {}", &key))
            .await?;
        dialogue.update(BotState::ReceiverMode(text, key)).await?;
    } else {
        bot.send_message(msg.chat.id, "Invalid repo name. please try again")
            .await?;
    }
    Ok(())
}

pub async fn receive_status(
    bot: Bot,
    dialogue: MyDialogue,
    (repo_name, key): (String, String),
    sqlite_pool: Pool<Sqlite>,
    msg: Message,
    cmd: RepoCommand,
) -> HandlerResult {
    match cmd {
        RepoCommand::Help => {
            bot.send_message(msg.chat.id, RepoCommand::descriptions().to_string())
                .await?
        }
        RepoCommand::GetInfo => {
            bot.send_message(msg.chat.id, format!("name: {}", repo_name))
                .await?;
            bot.send_message(msg.chat.id, format!("key: {}", key))
                .await?
        }
        RepoCommand::Status => {
            let record = query!(
                r#"
                SELECT status 
                FROM main.repos
                WHERE id = ?
                "#,
                key
            )
            .fetch_one(&sqlite_pool)
            .await?;
            bot.send_message(
                msg.chat.id,
                format!(
                    "status: {}",
                    DeployStatus::try_from(record.status.as_str())?
                ),
            )
            .await?
        }
        RepoCommand::Rename(name) => {
            let result = query!(
                r#"
                UPDATE main.repos
                SET name = ?
                WHERE id = ?
                "#,
                name,
                key
            )
            .execute(&sqlite_pool)
            .await?;
            info!(
                "Query success with affacted rows: {}",
                result.rows_affected()
            );
            dialogue.update(BotState::ReceiverMode(name, key)).await?;
            bot.send_message(msg.chat.id, "Successfully updated repo name")
                .await?
        }
        RepoCommand::Reset => {
            let result = query!(
                r#"
                DELETE FROM main.repos
                WHERE id = ?
                "#,
                key
            )
            .execute(&sqlite_pool)
            .await?;
            info!(
                "Query success with affacted rows: {}",
                result.rows_affected()
            );
            dialogue.reset().await?;
            bot.send_message(msg.chat.id, "Let's start by configuring repo name")
                .await?
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

use serde::{Deserialize, Serialize};
use std::fmt::Display;
use teloxide::macros::BotCommands;

use crate::app::util::error::ServiceError;

#[derive(Clone, Default, Serialize, Deserialize)]
pub enum BotState {
    #[default]
    Start,
    ConfigMode,
    ReceiverMode(String, String),
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum RepoCommand {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "display configured repo info.")]
    GetInfo,
    #[command(description = "get configured repo status.")]
    Status,
    #[command(description = "rename repo.")]
    Rename(String),
    #[command(description = "reset repo config.")]
    Reset,
}

#[derive(Default, Deserialize, sqlx::Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "status", rename_all = "UPPERCASE")]
pub enum DeployStatus {
    #[default]
    Idle,
    Deploy,
    Success,
    Failure,
}

impl Display for DeployStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Idle => "idle",
                Self::Deploy => "deploying",
                Self::Success => "deploy success",
                Self::Failure => "deploy failure",
            }
        )
    }
}

impl TryFrom<&str> for DeployStatus {
    type Error = ServiceError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "IDLE" => Ok(Self::Idle),
            "DEPLOY" => Ok(Self::Deploy),
            "SUCCESS" => Ok(Self::Success),
            "FAILURE" => Ok(Self::Failure),
            _ => Err(ServiceError::TryFrom {
                field: "status",
                from: value.to_string(),
                into: "DeployStatus",
                expect: "IDLE, DEPLOY, SUCCESS, or FAILURE",
            }),
        }
    }
}

use crate::app::util::error::ServiceError;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use teloxide::macros::BotCommands;

#[derive(Clone, Default, Serialize, Deserialize)]
pub enum BotState {
    #[default]
    Start,
    ConfigMode(Vec<String>, String),
    NormalMode(Vec<String>),
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "snake_case",
    description = "These commands are supported:"
)]
pub enum GeneralCommand {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "display all configured repos.")]
    List,
    #[command(description = "display all jobs that was created today.")]
    Today,
    #[command(
        description = "create new repo in the following format: /create <repo_name>\ni.e. /create Turbo Incubator Prototype"
    )]
    Create(String),
    #[command(
        description = "select repo for manipulation by index in the following format: /select_repo <index>\ni.e. /select_repo 1"
    )]
    SelectRepo(usize),
    #[command(description = "[DEBUG] Successfully reset all state.")]
    Reset,
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "snake_case",
    description = "These commands are supported:"
)]
pub enum RepoCommand {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "display current repo info.")]
    GetInfo,
    #[command(description = "display all jobs that was created today for current repo.")]
    Today,
    #[command(description = "display all running jobs for current repo.")]
    Running,
    #[command(description = "get latest jobs created for this repo.")]
    Latest,
    #[command(description = "rename current repo.")]
    Rename(String),
    #[command(description = "delete selected repo.")]
    Delete,
    #[command(description = "deselect current repo for manipulation.")]
    Cancel,
}

#[derive(Default, Deserialize, sqlx::Type, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "status", rename_all = "UPPERCASE")]
pub enum DeployStatus {
    #[default]
    Running,
    Cancelled,
    Success,
    Failure,
}

impl Display for DeployStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Running => "RUNNING",
                Self::Cancelled => "CANCELLED",
                Self::Success => "SUCCESS",
                Self::Failure => "FAILURE",
            }
        )
    }
}

impl TryFrom<&str> for DeployStatus {
    type Error = ServiceError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "RUNNING" => Ok(Self::Running),
            "CANCELLED" => Ok(Self::Cancelled),
            "SUCCESS" => Ok(Self::Success),
            "FAILURE" => Ok(Self::Failure),
            _ => Err(ServiceError::TryFrom {
                field: "status",
                from: value.to_string(),
                into: "DeployStatus",
                expect: "RUNNING, CANCELLED, SUCCESS, or FAILURE",
            }),
        }
    }
}

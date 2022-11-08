use crate::app::util::sentry::*;
use axum::{response::IntoResponse, Json};
use http::header::ToStrError;
use hyper::StatusCode;
use sentry::capture_error as capture_exception;
use serde::Serialize;
use teloxide::RequestError;
use tracing::{error, warn};

#[derive(thiserror::Error, Debug, Serialize)]
#[serde(tag = "code", content = "description")]
#[serde(rename_all(serialize = "SCREAMING_SNAKE_CASE"))]
#[non_exhaustive]
#[allow(dead_code)]
pub enum ServiceError {
    // #[error(transparent)]
    // Reqwest(#[from] reqwest::Error),
    // #[error(transparent)]
    // Redis(#[from] redis::RedisError),
    #[error(transparent)]
    #[serde(serialize_with = "as_json_string::serialize")]
    Sqlx(#[from] sqlx::Error),
    #[error("{0} middleware not set")]
    MiddlewareNotSet(&'static str),
    #[error("app config not set")]
    ConfigNotSet,
    #[error("Rc still has more than 0 reference(s). This is a bug")]
    RcHasReference,
    #[error("error: failed to parse error message: {0}")]
    ParseMessage(String),
    #[error("failed to validate request data: {field} reason: {reason}")]
    ValidateFailure { field: &'static str, reason: String },
    #[error(transparent)]
    #[serde(serialize_with = "as_json_string::serialize")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error(transparent)]
    #[serde(serialize_with = "as_json_string::serialize")]
    ParseUtf8(#[from] std::str::Utf8Error),
    #[error("bad credential")]
    BadCredential,
    #[error("rejected reason: {0}")]
    Rejected(String),
    #[error(transparent)]
    #[serde(serialize_with = "as_json_string::serialize")]
    Uuid(#[from] uuid::Error),
    #[error("failed to convert from '{field}' value '{from}' into '{into}' expecting {expect}")]
    TryFrom {
        field: &'static str,
        from: String,
        into: &'static str,
        expect: &'static str,
    },
    // #[error(transparent)]
    // Validation(#[from] validator::ValidationError),
    // #[error(transparent)]
    // Validations(#[from] validator::ValidationErrors),
    // #[error(transparent)]
    // Base64Decode(#[from] base64::DecodeError),
    #[error("empty value at index: {0}")]
    EmptySliceIndex(usize),
    #[error("failed to send a value")]
    SendError,
    #[error(transparent)]
    #[serde(serialize_with = "as_json_string::serialize")]
    OneshotRecvError(#[from] tokio::sync::oneshot::error::RecvError),
    #[error(transparent)]
    #[serde(serialize_with = "as_json_string::serialize")]
    TaskJoinError(#[from] tokio::task::JoinError),
    #[error(transparent)]
    #[serde(serialize_with = "as_json_string::serialize")]
    SemaphoreAquire(#[from] tokio::sync::AcquireError),
    #[error("client response timeout")]
    ClientTimeout,
    // #[error(transparent)]
    // CookieParse(#[from] cookie::ParseError),
    // #[error(transparent)]
    // HttpHeader(#[from] http::header::ToStrError),
    #[error("http header not found")]
    HttpHeaderNotFound,
    #[error(transparent)]
    #[serde(serialize_with = "as_json_string::serialize")]
    ToStrError(#[from] ToStrError),
    #[error(transparent)]
    #[serde(serialize_with = "as_json_string::serialize")]
    TeloxideError(#[from] RequestError),
    #[error(transparent)]
    #[serde(serialize_with = "as_json_string::serialize")]
    ServerError(#[from] axum::Error),
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for ServiceError {
    fn from(_: tokio::sync::mpsc::error::SendError<T>) -> Self {
        ServiceError::SendError
    }
}

impl From<()> for ServiceError {
    fn from(_: ()) -> Self {
        ServiceError::SendError
    }
}

impl ServiceError {
    pub fn get_code(&self) -> StatusCode {
        match self {
            // Self::Reqwest(e) if e.is_body() => {
            //     warn!("reqwest body failed: {:?}", e);
            //     capture_warning("HTTP client failed to parse payload body");
            //     Code::InvalidArgument
            // }
            // Self::Reqwest(e) if e.is_builder() => {
            //     error!("reqwest builder failed: {:?}", e);
            //     capture_fatal("HTTP client failed to be initialized");
            //     Code::Internal
            // }
            // Self::Reqwest(e) if e.is_connect() => {
            //     error!("reqwest connection failed: {:?}", e);
            //     capture_error("HTTP client failed to initiate connection");
            //     Code::Unavailable
            // }
            // Self::Reqwest(e) if e.is_decode() => {
            //     warn!("reqwest decode failed: {:?}", e);
            //     capture_warning("HTTP client failed to decode payload instance");
            //     Code::FailedPrecondition
            // }
            // Self::Reqwest(e) if e.is_redirect() => {
            //     warn!("reqwest redirect failed: {:?}", e);
            //     capture_warning("HTTP client failed to perform redirection");
            //     Code::Internal
            // }
            // Self::Reqwest(e) if e.is_timeout() => {
            //     warn!("reqwest timed out: {:?}", e);
            //     capture_warning("HTTP client timed-out");
            //     Code::DeadlineExceeded
            // }
            // Self::Reqwest(e) if e.is_request() => {
            //     error!("bad reqwest: {:?}", e);
            //     capture_error("HTTP client threw internal exception");
            //     Code::FailedPrecondition
            // }
            // Self::Reqwest(e) if e.is_status() => {
            //     warn!("reqwest status failed: {:?}", e);
            //     capture_warning("HTTP client received bad protocol status");
            //     Code::FailedPrecondition
            // }
            // Self::Reqwest(e) => {
            //     error!("general reqwest error: {:?}", e);
            //     capture_error("HTTP client returned general failure");
            //     Code::Internal
            // }
            // Self::Redis(e) if e.is_timeout() => {
            //     warn!("redis timed out: {:?}", e);
            //     capture_warning("Redis client timed-out");
            //     Code::FailedPrecondition
            // }
            // Self::Redis(e) if e.is_cluster_error() => {
            //     error!("redis cluster failed: {:?}", e);
            //     capture_error("Redis client returned cluster related exception");
            //     Code::Unavailable
            // }
            // Self::Redis(e) if e.is_connection_dropped() => {
            //     warn!("redis connection dropped: {:?}", e);
            //     capture_warning("Redis client unexpectedly dropped connection");
            //     Code::Unavailable
            // }
            // Self::Redis(e) if e.is_connection_refusal() => {
            //     error!("redis connection refused: {:?}", e);
            //     capture_error("Redis client encountered connection refusal");
            //     Code::Unavailable
            // }
            // Self::Redis(e) if e.is_io_error() => {
            //     error!("redis io error: {:?}", e);
            //     capture_fatal("Redis client encountered network io failure");
            //     Code::Internal
            // }
            // Self::Redis(e) => {
            //     error!("general redis error: {:?}", e);
            //     capture_warning("Redis client returned internal exception");
            //     Code::Unavailable
            // }
            Self::Sqlx(e) => match e.as_database_error() {
                Some(e) => {
                    warn!(
                        "sqlite threw an exception: (code: {}) {}",
                        e.code().map_or("NONE".to_owned(), |v| v.to_string()),
                        e.message()
                    );
                    capture_warning("sqlite threw an exception");
                    StatusCode::BAD_REQUEST
                }
                // Some(e) => match e.try_downcast_ref::<sqlx::sqlite::SqliteError>() {
                //     Some(e) => {
                //         error!("sqlite threw an exception: {:?}", e);
                //         capture_fatal("Sqlx driver threw an exception");
                //         StatusCode::INTERNAL_SERVER_ERROR
                //     }
                //     e => {
                //         error!("sqlx threw an exception: {:?}", e);
                //         capture_fatal("Sqlx driver threw an exception");
                //         StatusCode::INTERNAL_SERVER_ERROR
                //     }
                // },
                None if &format!("{:?}", e) == "RowNotFound" => {
                    warn!("database return zero result. expecting one: {:?}", e);
                    StatusCode::NOT_FOUND
                }
                None => {
                    error!("sqlx threw an exception: {:?}", e);
                    capture_fatal("Sqlx driver threw an exception");
                    StatusCode::INTERNAL_SERVER_ERROR
                }
            },
            Self::MiddlewareNotSet(e) => {
                error!("middleware not set: {:?}", e);
                capture_fatal("Service middleware was not properly setup");
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::ConfigNotSet => {
                error!("config was not set");
                capture_fatal("Service configuration was not properly setup");
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::ParseMessage(e) => {
                error!("error: failed to parse error message: {}", e);
                capture_error(
                    "Service encountered failure while attempting to parse error response message",
                );
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::RcHasReference => {
                error!("{:?}", self);
                capture_fatal(
                    "Reference counted memory returned exception when attempting downgrade to non-reference counted memory",
                );
                StatusCode::INTERNAL_SERVER_ERROR
            }
            // Self::ValidateFailure { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            Self::ParseInt(e) => {
                warn!("failed integer parsing: {:?}", e);
                capture_warning(
                    "Service encountered failure while attempting to parse input to the integer",
                );
                StatusCode::UNPROCESSABLE_ENTITY
            }
            Self::ParseUtf8(e) => {
                warn!("utf8 encoding failed: {:?}", e);
                capture_warning(
                    "Service encountered failure while attempting to parse input to the UTF-8 encoded string",
                );
                StatusCode::UNPROCESSABLE_ENTITY
            }
            Self::BadCredential => StatusCode::UNAUTHORIZED,
            Self::Rejected(e) => {
                warn!("access rejected reason: {}", e);
                capture_warning(
                    "Incoming gRPC request attempted to access non-authorized resource",
                );
                StatusCode::FORBIDDEN
            }
            Self::Uuid(e) => {
                warn!("uuid parsing failure: {:?}", e);
                capture_warning(
                    "Service encountered failure while attempting to parse input to UUID",
                );
                StatusCode::UNPROCESSABLE_ENTITY
            }
            Self::TryFrom { .. } => StatusCode::BAD_REQUEST,
            // Self::Validation(_) => Code::InvalidArgument,
            // Self::Validations(_) => Code::InvalidArgument,
            // Self::Base64Decode(e) => {
            //     warn!("base64 decode failure: {:?}", e);
            //     capture_warning(
            //         "Service encountered failure from cryptography library `base64` failure to decode input",
            //     );
            //     Code::FailedPrecondition
            // }
            Self::EmptySliceIndex(i) => {
                warn!("empty slice index at: {}", i);
                capture_warning(
                    "Service encountered failure while attempting to access non-existing slice index",
                );
                StatusCode::BAD_REQUEST
            }
            Self::SendError => {
                warn!("tokio signal send error");
                capture_warning(
                    "Service encountered failure while attempting to send signal across asynchrous task",
                );
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::OneshotRecvError(e) => {
                warn!("tokio oneshot recv error: {:?}", e);
                capture_warning(
                    "Service encountered failure while attempting to receive signal from oneshot channel",
                );
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::TaskJoinError(e) if e.is_panic() => {
                error!("tokio task panicked: {:?}", e);
                {
                    capture_exception(e);
                    capture_fatal(
                        "Service encountered failure from unhandled exception inside asynchronous task",
                    );
                }
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::TaskJoinError(e) if !e.is_cancelled() => {
                warn!("tokio task returned error: {:?}", e);
                capture_warning("Service encountered failure while computing asynchronous task");
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::TaskJoinError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::SemaphoreAquire(e) => {
                warn!("semaphore acquire error: {:?}", e);
                capture_warning(
                    "Service encountered failure while attempting to acquire semaphore permit",
                );
                StatusCode::INTERNAL_SERVER_ERROR
            }
            // // total combined deadline time should not exceed retry count reset timer
            // Self::QueueDeclareTimeout => Code::DeadlineExceeded,
            // Self::QueueBindTimeout => Code::DeadlineExceeded,
            // Self::QueueBasicConsumeTimeout => Code::DeadlineExceeded,
            // Self::QueueBasicAckTimeout => Code::DeadlineExceeded,
            Self::ClientTimeout => StatusCode::REQUEST_TIMEOUT,
            // Self::CookieParse(e) => {
            //     warn!("cookie parse error: {:?}", e);
            //     capture_warning(
            //         "Service encountered failure while attempting to parse http cookie",
            //     );
            //     Code::FailedPrecondition
            // }
            // Self::HttpHeader(e) => {
            //     warn!("http header error: {:?}", e);
            //     capture_warning(
            //         "Service encountered failure while attempting to parse http header",
            //     );
            //     Code::FailedPrecondition
            // }
            Self::HttpHeaderNotFound => {
                warn!("http header error");
                capture_warning(
                    "Service encountered failure while attempting to access request header",
                );
                StatusCode::EXPECTATION_FAILED
            }
            Self::TeloxideError(e) => {
                error!("teloxide error: {:?}", e);
                capture_warning(
                    "Service encountered unexpected failure while interacting with Telegram service",
                );
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::ServerError(e) => {
                error!("axum error: {:?}", e);
                capture_warning(
                    "Service encountered unexpected failure while processing request in general",
                );
                StatusCode::INTERNAL_SERVER_ERROR
            }
            #[allow(unreachable_patterns)]
            _ => {
                error!("undocumented error: {}", &self.to_string());
                capture_fatal("Service encountered exception from unknown source");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> axum::response::Response {
        (self.get_code(), Json(self)).into_response()
    }
}

mod as_json_string {
    use serde::ser::{Serialize, Serializer};

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: ToString,
        S: Serializer,
    {
        let j = value.to_string();
        j.serialize(serializer)
    }
}

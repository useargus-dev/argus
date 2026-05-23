use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{message}")]
    Message { code: &'static str, message: String },

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    pub fn message(code: &'static str, message: impl Into<String>) -> Self {
        Self::Message {
            code,
            message: message.into(),
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            AppError::Message { code, .. } => code,
            AppError::Internal(_) => "INTERNAL_ERROR",
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub second_factor_type: Option<String>,
}

impl From<AppError> for ErrorPayload {
    fn from(err: AppError) -> Self {
        ErrorPayload {
            code: err.code().to_string(),
            message: err.to_string(),
            second_factor_type: None,
        }
    }
}

impl From<AppError> for String {
    fn from(err: AppError) -> Self {
        let msg = err.to_string();
        serde_json::to_string(&ErrorPayload {
            code: err.code().to_string(),
            message: msg.clone(),
            second_factor_type: None,
        })
        .unwrap_or(msg)
    }
}

pub type AppResult<T> = Result<T, AppError>;

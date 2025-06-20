use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use org_parser::JsonConversionError;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{error, warn};

use crate::file_resolver::FileResolverError;

/// API エラーレスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiErrorResponse {
    pub error: String,
    pub message: String,
    pub code: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// API エラー型
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("File resolver error: {source}")]
    FileResolver {
        #[from]
        source: FileResolverError,
    },

    #[error("JSON conversion error: {source}")]
    JsonConversion {
        #[from]
        source: JsonConversionError,
    },

    #[error("File parsing error: {message}")]
    FileParsing { message: String },

    #[error("Invalid query parameter: {parameter} = {value}")]
    InvalidQueryParameter { parameter: String, value: String },

    #[error("Internal server error: {message}")]
    Internal { message: String },

    #[error("IO error: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },
}

impl ApiError {
    /// エラーの詳細情報を取得
    pub fn details(&self) -> Option<String> {
        match self {
            ApiError::FileResolver { source } => Some(format!("File resolver: {}", source)),
            ApiError::JsonConversion { source } => Some(format!("JSON conversion: {}", source)),
            ApiError::FileParsing { message } => Some(message.clone()),
            ApiError::InvalidQueryParameter { parameter, value } => Some(format!(
                "Parameter '{}' has invalid value '{}'",
                parameter, value
            )),
            ApiError::Internal { message } => Some(message.clone()),
            ApiError::Io { source } => Some(format!("IO error: {}", source)),
        }
    }

    /// HTTPステータスコードを取得
    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiError::FileResolver { source } => match source {
                FileResolverError::FileNotFound { .. } => StatusCode::NOT_FOUND,
                FileResolverError::InvalidPath { .. }
                | FileResolverError::InvalidFileExtension
                | FileResolverError::PathTraversalDetected
                | FileResolverError::AbsolutePathNotAllowed => StatusCode::BAD_REQUEST,
                FileResolverError::IoError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            },
            ApiError::JsonConversion { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::FileParsing { .. } => StatusCode::BAD_REQUEST,
            ApiError::InvalidQueryParameter { .. } => StatusCode::BAD_REQUEST,
            ApiError::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Io { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// エラーコードを取得
    pub fn error_code(&self) -> &'static str {
        match self {
            ApiError::FileResolver { source } => match source {
                FileResolverError::FileNotFound { .. } => "FILE_NOT_FOUND",
                FileResolverError::InvalidPath { .. } => "INVALID_PATH",
                FileResolverError::InvalidFileExtension => "INVALID_FILE_EXTENSION",
                FileResolverError::PathTraversalDetected => "PATH_TRAVERSAL_DETECTED",
                FileResolverError::AbsolutePathNotAllowed => "ABSOLUTE_PATH_NOT_ALLOWED",
                FileResolverError::IoError { .. } => "IO_ERROR",
            },
            ApiError::JsonConversion { .. } => "JSON_CONVERSION_ERROR",
            ApiError::FileParsing { .. } => "FILE_PARSING_ERROR",
            ApiError::InvalidQueryParameter { .. } => "INVALID_QUERY_PARAMETER",
            ApiError::Internal { .. } => "INTERNAL_SERVER_ERROR",
            ApiError::Io { .. } => "IO_ERROR",
        }
    }

    /// ログレベルを決定
    pub fn should_log_as_error(&self) -> bool {
        matches!(self, ApiError::Internal { .. } | ApiError::Io { .. })
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_response = ApiErrorResponse {
            error: self.error_code().to_string(),
            message: self.to_string(),
            code: status.as_u16(),
            details: self.details(),
        };

        // ログ出力
        if self.should_log_as_error() {
            error!("API Error: {} - {}", self.error_code(), self);
        } else {
            warn!("API Warning: {} - {}", self.error_code(), self);
        }

        (status, Json(error_response)).into_response()
    }
}

/// Result型のエイリアス
pub type ApiResult<T> = Result<T, ApiError>;

/// 便利な変換関数
impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::Internal {
            message: err.to_string(),
        }
    }
}

impl From<String> for ApiError {
    fn from(message: String) -> Self {
        ApiError::Internal { message }
    }
}

impl From<&str> for ApiError {
    fn from(message: &str) -> Self {
        ApiError::Internal {
            message: message.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_error_status_codes() {
        // FileNotFound -> 404
        let error = ApiError::FileResolver {
            source: FileResolverError::FileNotFound {
                filepath: "test.org".to_string(),
            },
        };
        assert_eq!(error.status_code(), StatusCode::NOT_FOUND);

        // InvalidPath -> 400
        let error = ApiError::FileResolver {
            source: FileResolverError::InvalidPath {
                message: "test".to_string(),
            },
        };
        assert_eq!(error.status_code(), StatusCode::BAD_REQUEST);

        // Internal -> 500
        let error = ApiError::Internal {
            message: "test".to_string(),
        };
        assert_eq!(error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_api_error_codes() {
        let error = ApiError::FileResolver {
            source: FileResolverError::FileNotFound {
                filepath: "test.org".to_string(),
            },
        };
        assert_eq!(error.error_code(), "FILE_NOT_FOUND");

        let error = ApiError::InvalidQueryParameter {
            parameter: "max_depth".to_string(),
            value: "invalid".to_string(),
        };
        assert_eq!(error.error_code(), "INVALID_QUERY_PARAMETER");
    }

    #[test]
    fn test_api_error_logging_level() {
        // Internal errors should be logged as errors
        let error = ApiError::Internal {
            message: "test".to_string(),
        };
        assert!(error.should_log_as_error());

        // Client errors should be logged as warnings
        let error = ApiError::FileResolver {
            source: FileResolverError::FileNotFound {
                filepath: "test.org".to_string(),
            },
        };
        assert!(!error.should_log_as_error());
    }

    #[test]
    fn test_api_error_response_serialization() {
        let response = ApiErrorResponse {
            error: "TEST_ERROR".to_string(),
            message: "Test message".to_string(),
            code: 400,
            details: Some("Test details".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("TEST_ERROR"));
        assert!(json.contains("Test message"));
        assert!(json.contains("400"));
        assert!(json.contains("Test details"));
    }
}

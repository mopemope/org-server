use crate::{
    api_error::{ApiError, ApiResult},
    config::Config,
    file_resolver::FileResolver,
    parse::parse_org_file,
};
use anyhow::Result;
use axum::{
    Router,
    extract::{Path, Query, State},
    response::Json as AxumJson,
    routing::{get, post},
};
use org_parser::JsonConversionConfig;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// アプリケーション状態
#[derive(Clone)]
pub struct AppState {
    pub file_resolver: Arc<FileResolver>,
}

/// クエリパラメータ
#[derive(Debug, Deserialize)]
pub struct OrgFileQuery {
    /// 美しい整形を行うかどうか
    #[serde(default)]
    pub pretty: bool,
    /// 位置情報を含めるかどうか
    #[serde(default)]
    pub include_position: bool,
    /// 空のセクションを含めるかどうか
    #[serde(default = "default_include_empty_sections")]
    pub include_empty_sections: bool,
    /// 最大深度制限
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
}

fn default_include_empty_sections() -> bool {
    true
}

fn default_max_depth() -> usize {
    10
}

impl OrgFileQuery {
    /// JsonConversionConfigに変換
    pub fn to_json_config(&self) -> JsonConversionConfig {
        JsonConversionConfig {
            max_depth: self.max_depth,
            include_position: self.include_position,
            pretty_print: self.pretty,
            include_empty_sections: self.include_empty_sections,
        }
    }
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    match tokio::signal::ctrl_c().await {
        Ok(()) => {
            info!("Received shutdown signal, shutting down gracefully...");
        }
        Err(err) => {
            tracing::error!("Failed to listen for shutdown signal: {}", err);
            // Fall through — without a signal we just let the future complete,
            // which will still trigger the shutdown path.
        }
    }
}

/// サーバー起動
pub async fn run_server(
    port: u16,
    _config: Config,
    file_resolver: Arc<FileResolver>,
) -> Result<()> {
    let app_state = AppState { file_resolver };

    // build our application with routes
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/api/orgs/{*filepath}", get(get_org_file))
        .route("/api/edit/todo/{*filepath}", post(update_todo_status))
        .route("/api/edit/append/{*filepath}", post(append_task))
        .route("/api/edit/schedule/{*filepath}", post(update_scheduling))
        .with_state(app_state);

    // Try to bind to the specified port, with fallback options
    let mut current_port = port;
    let max_attempts = 10;

    for attempt in 0..max_attempts {
        let addr = format!("0.0.0.0:{}", current_port);
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(listener) => {
                if current_port != port {
                    warn!(
                        "Original port {} was in use, using port {} instead",
                        port, current_port
                    );
                }
                info!("Server starting on {}", addr);
                info!("API endpoints:");
                info!("  GET /api/orgs/{{filepath}} - Get org file as JSON");
                info!("  GET /health - Health check");
                info!("  Example: GET /api/orgs/my-notes.org?pretty=true");
                info!("Press Ctrl+C to stop the server");
                axum::serve(listener, app)
                    .with_graceful_shutdown(shutdown_signal())
                    .await?;
                info!("Server shutdown complete");
                return Ok(());
            }
            Err(err) => {
                if attempt == max_attempts - 1 {
                    return Err(err.into());
                }
                warn!(
                    "Port {} is in use, trying port {}",
                    current_port,
                    current_port + 1
                );
                current_port += 1;
            }
        }
    }

    unreachable!()
}

/// ルートハンドラー
async fn root() -> &'static str {
    "Org Server API\n\nAvailable endpoints:\n- GET /api/orgs/{filepath} - Get org file as JSON\n- GET /health - Health check"
}

/// ヘルスチェックハンドラー
async fn health() -> AxumJson<serde_json::Value> {
    AxumJson(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Orgファイル取得ハンドラー
async fn get_org_file(
    State(state): State<AppState>,
    Path(filepath): Path<String>,
    Query(query): Query<OrgFileQuery>,
) -> ApiResult<AxumJson<Box<serde_json::value::RawValue>>> {
    debug!("GET /api/orgs/{} with query: {:?}", filepath, query);

    // クエリパラメータの検証
    validate_query_parameters(&query)?;

    // ファイルパスの解決
    let resolved_path = state.file_resolver.resolve_file(&filepath).await?;
    debug!("Resolved file path: {}", resolved_path.display());

    // Orgファイルのパース
    let org = parse_org_file(&resolved_path)
        .await
        .map_err(|e| ApiError::FileParsing {
            message: format!("Failed to parse org file '{}': {}", filepath, e),
        })?;

    // JSON変換設定
    let json_config = query.to_json_config();
    debug!("JSON conversion config: {:?}", json_config);

    // JSON変換
    let json_string = org.to_json_with_config(&json_config)?;
    let raw_value = serde_json::value::RawValue::from_string(json_string).map_err(|e| {
        ApiError::JsonConversion {
            source: org_parser::JsonConversionError::SerializationError { source: e },
        }
    })?;

    debug!("Successfully converted org file to JSON: {}", filepath);
    Ok(AxumJson(raw_value))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateTodoStatusRequest {
    pub headline_line_number: usize,
    pub new_status: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AppendTaskRequest {
    pub title: String,
    pub status: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateSchedulingRequest {
    pub headline_line_number: usize,
    pub scheduling_type: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct WriteResponse {
    pub success: bool,
    pub message: String,
}

/// TODOステータス更新ハンドラー
async fn update_todo_status(
    State(state): State<AppState>,
    Path(filepath): Path<String>,
    AxumJson(payload): AxumJson<UpdateTodoStatusRequest>,
) -> ApiResult<AxumJson<WriteResponse>> {
    debug!("POST /api/orgs/{}/todo", filepath);
    let resolved = state.file_resolver.resolve_file(&filepath).await?;

    match crate::edit::do_update_todo_status(
        &resolved,
        payload.headline_line_number,
        &payload.new_status,
    )
    .await
    {
        Ok(msg) => Ok(AxumJson(WriteResponse {
            success: true,
            message: msg,
        })),
        Err(e) => Err(ApiError::Internal {
            message: e.to_string(),
        }),
    }
}

/// タスク追記ハンドラー
async fn append_task(
    State(state): State<AppState>,
    Path(filepath): Path<String>,
    AxumJson(payload): AxumJson<AppendTaskRequest>,
) -> ApiResult<AxumJson<WriteResponse>> {
    debug!("POST /api/orgs/{}/append", filepath);
    let resolved = state.file_resolver.resolve_file(&filepath).await?;

    match crate::edit::do_append_task(
        &resolved,
        &payload.title,
        payload.status.as_deref(),
        payload.tags.as_deref(),
    )
    .await
    {
        Ok(msg) => Ok(AxumJson(WriteResponse {
            success: true,
            message: msg,
        })),
        Err(e) => Err(ApiError::Internal {
            message: e.to_string(),
        }),
    }
}

/// スケジューリング更新ハンドラー
async fn update_scheduling(
    State(state): State<AppState>,
    Path(filepath): Path<String>,
    AxumJson(payload): AxumJson<UpdateSchedulingRequest>,
) -> ApiResult<AxumJson<WriteResponse>> {
    debug!("POST /api/orgs/{}/schedule", filepath);
    let resolved = state.file_resolver.resolve_file(&filepath).await?;

    match crate::edit::do_update_scheduling(
        &resolved,
        payload.headline_line_number,
        &payload.scheduling_type,
        &payload.timestamp,
    )
    .await
    {
        Ok(msg) => Ok(AxumJson(WriteResponse {
            success: true,
            message: msg,
        })),
        Err(e) => Err(ApiError::Internal {
            message: e.to_string(),
        }),
    }
}

/// クエリパラメータの検証
fn validate_query_parameters(query: &OrgFileQuery) -> ApiResult<()> {
    // max_depthの範囲チェック
    if query.max_depth == 0 || query.max_depth > 100 {
        return Err(ApiError::InvalidQueryParameter {
            parameter: "max_depth".to_string(),
            value: query.max_depth.to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use serde_json::Value;
    use tempfile::TempDir;
    use tokio::fs;
    use tower::ServiceExt;

    fn create_test_config(org_paths: Vec<String>) -> Config {
        Config {
            org_path: org_paths,
            server_port: 3000,
            mcp_host: "127.0.0.1".to_string(),
            mcp_port: 3001,
        }
    }

    async fn create_test_app(config: Config) -> Router {
        let file_resolver = Arc::new(FileResolver::new(&config));
        let app_state = AppState { file_resolver };

        Router::new()
            .route("/", get(root))
            .route("/api/orgs/{*filepath}", get(get_org_file))
            .route("/api/edit/todo/{*filepath}", post(update_todo_status))
            .route("/api/edit/append/{*filepath}", post(append_task))
            .route("/api/edit/schedule/{*filepath}", post(update_scheduling))
            .with_state(app_state)
    }

    #[tokio::test]
    async fn test_root_endpoint() {
        let config = create_test_config(vec![]);
        let app = create_test_app(config).await;

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_org_file_success() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().to_string_lossy().to_string();

        // テストファイルを作成
        let test_content = r#"#+TITLE: Test Document
#+AUTHOR: Test Author

* First Section
This is the first section.

** Subsection
This is a subsection.
"#;
        let test_file_path = temp_dir.path().join("test.org");
        fs::write(&test_file_path, test_content).await?;

        let config = create_test_config(vec![temp_path]);
        let app = create_test_app(config).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/orgs/test.org")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
        let json: Value = serde_json::from_slice(&body)?;

        // JSONの基本構造を確認
        assert!(json.get("title").is_some());
        assert!(json.get("sections").is_some());

        Ok(())
    }

    #[tokio::test]
    async fn test_get_org_file_not_found() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().to_string_lossy().to_string();

        let config = create_test_config(vec![temp_path]);
        let app = create_test_app(config).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/orgs/nonexistent.org")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_org_file_with_query_params() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().to_string_lossy().to_string();

        // テストファイルを作成
        let test_file_path = temp_dir.path().join("test.org");
        fs::write(&test_file_path, "* Test").await?;

        let config = create_test_config(vec![temp_path]);
        let app = create_test_app(config).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/orgs/test.org?pretty=true&max_depth=5")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_org_file_invalid_extension() -> Result<()> {
        let config = create_test_config(vec!["/tmp".to_string()]);
        let app = create_test_app(config).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/orgs/test.txt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_org_file_path_traversal() -> Result<()> {
        let config = create_test_config(vec!["/tmp".to_string()]);
        let app = create_test_app(config).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/orgs/../etc/passwd.org")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        Ok(())
    }

    #[tokio::test]
    async fn test_query_parameter_validation() {
        let query = OrgFileQuery {
            pretty: true,
            include_position: false,
            include_empty_sections: true,
            max_depth: 5,
        };
        assert!(validate_query_parameters(&query).is_ok());

        // 無効なmax_depth
        let query = OrgFileQuery {
            pretty: true,
            include_position: false,
            include_empty_sections: true,
            max_depth: 0,
        };
        assert!(validate_query_parameters(&query).is_err());

        let query = OrgFileQuery {
            pretty: true,
            include_position: false,
            include_empty_sections: true,
            max_depth: 101,
        };
        assert!(validate_query_parameters(&query).is_err());
    }

    #[tokio::test]
    async fn test_update_todo_status_endpoint() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().to_string_lossy().to_string();
        let test_file_path = temp_dir.path().join("test_write.org");
        fs::write(&test_file_path, "* TODO Test Task\n").await?;

        let config = create_test_config(vec![temp_path]);
        let app = create_test_app(config).await;

        let payload = UpdateTodoStatusRequest {
            headline_line_number: 1,
            new_status: "DONE".to_string(),
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/edit/todo/test_write.org")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&payload)?))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let content = fs::read_to_string(&test_file_path).await?;
        assert_eq!(content, "* DONE Test Task\n");
        Ok(())
    }

    #[tokio::test]
    async fn test_append_task_endpoint() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().to_string_lossy().to_string();
        let test_file_path = temp_dir.path().join("test_append.org");
        fs::write(&test_file_path, "* Existing Task\n").await?;

        let config = create_test_config(vec![temp_path]);
        let app = create_test_app(config).await;

        let payload = AppendTaskRequest {
            title: "New Task".to_string(),
            status: Some("TODO".to_string()),
            tags: Some(vec!["work".to_string()]),
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/edit/append/test_append.org")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&payload)?))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let content = fs::read_to_string(&test_file_path).await?;
        assert!(content.contains("* TODO New Task :work:"));
        Ok(())
    }

    #[tokio::test]
    async fn test_update_scheduling_endpoint() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().to_string_lossy().to_string();
        let test_file_path = temp_dir.path().join("test_schedule.org");
        fs::write(&test_file_path, "* Task Setup\n").await?;

        let config = create_test_config(vec![temp_path]);
        let app = create_test_app(config).await;

        let payload = UpdateSchedulingRequest {
            headline_line_number: 1,
            scheduling_type: "SCHEDULED".to_string(),
            timestamp: "<2026-03-01 Sun>".to_string(),
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/edit/schedule/test_schedule.org")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&payload)?))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let content = fs::read_to_string(&test_file_path).await?;
        assert!(content.contains("SCHEDULED: <2026-03-01 Sun>"));
        Ok(())
    }
}

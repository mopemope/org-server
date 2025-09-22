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
    response::Json,
    routing::get,
};
use org_parser::JsonConversionConfig;
use serde::Deserialize;
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

/// サーバー起動
pub async fn run_server(
    port: u32,
    _config: Config,
    file_resolver: Arc<FileResolver>,
) -> Result<()> {
    let app_state = AppState { file_resolver };

    // build our application with routes
    let app = Router::new()
        .route("/", get(root))
        .route("/api/orgs/{*filepath}", get(get_org_file))
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
                info!("  Example: GET /api/orgs/my-notes.org?pretty=true");
                axum::serve(listener, app).await?;
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
    "Org Server API\n\nAvailable endpoints:\n- GET /api/orgs/{filepath} - Get org file as JSON"
}

/// Orgファイル取得ハンドラー
async fn get_org_file(
    State(state): State<AppState>,
    Path(filepath): Path<String>,
    Query(query): Query<OrgFileQuery>,
) -> ApiResult<Json<serde_json::Value>> {
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
    let json_value: serde_json::Value =
        serde_json::from_str(&json_string).map_err(|e| ApiError::JsonConversion {
            source: org_parser::JsonConversionError::SerializationError { source: e },
        })?;

    debug!("Successfully converted org file to JSON: {}", filepath);
    Ok(Json(json_value))
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
}

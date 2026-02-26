use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result as AnyResult};
use rmcp::{
    ErrorData as McpError, Json,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ServerCapabilities, ServerInfo},
    schemars::{self, JsonSchema},
    serde::{Deserialize, Serialize},
    tool, tool_handler, tool_router,
    transport::{SseServer, sse_server::SseServerConfig},
};
use tokio::fs;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};
use walkdir::WalkDir;

use crate::{
    config::Config,
    file_resolver::{FileResolver, FileResolverError},
};

#[derive(Clone)]
pub struct OrgMcpServer {
    file_resolver: Arc<FileResolver>,
    org_paths: Arc<Vec<PathBuf>>,
    tool_router: ToolRouter<Self>,
}

impl OrgMcpServer {
    pub fn new(file_resolver: Arc<FileResolver>, config: &Config) -> Self {
        let org_paths = config
            .org_path
            .iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();

        Self {
            file_resolver,
            org_paths: Arc::new(org_paths),
            tool_router: Self::tool_router(),
        }
    }

    fn resolve_relative_path(&self, path: &Path) -> String {
        for base in self.org_paths.iter() {
            if let Ok(relative) = path.strip_prefix(base) {
                return relative.display().to_string();
            }
        }
        path.display().to_string()
    }

    async fn collect_search_matches(
        &self,
        query: &str,
        case_sensitive: bool,
        limit: usize,
    ) -> AnyResult<Vec<SearchMatch>> {
        const MAX_MATCHES: usize = 100;
        let limit = limit.clamp(1, MAX_MATCHES);
        let needle = if case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };

        let mut matches = Vec::new();
        'path_loop: for base in self.org_paths.iter() {
            for entry in WalkDir::new(base).into_iter().filter_map(Result::ok) {
                if !entry.file_type().is_file() {
                    continue;
                }
                if entry.path().extension().and_then(|ext| ext.to_str()) != Some("org") {
                    continue;
                }

                let path = entry.into_path();
                let Ok(contents) = fs::read_to_string(&path).await else {
                    warn!(file = %path.display(), "Failed to read org file while searching");
                    continue;
                };

                let relative = self.resolve_relative_path(&path);

                for (idx, line) in contents.lines().enumerate() {
                    let haystack = if case_sensitive {
                        line.to_string()
                    } else {
                        line.to_lowercase()
                    };
                    if haystack.contains(&needle) {
                        matches.push(SearchMatch {
                            file: relative.clone(),
                            line: idx + 1,
                            snippet: line.trim().to_string(),
                        });
                        if matches.len() >= limit {
                            break 'path_loop;
                        }
                    }
                }

                if matches.len() >= limit {
                    break 'path_loop;
                }
            }
        }

        debug!(%query, results = matches.len(), "search completed");
        Ok(matches)
    }

    /// Walk all managed org files and parse them, yielding (Org, Path) pairs.
    async fn walk_org_files(&self) -> Vec<(org_parser::Org, PathBuf)> {
        let mut results = Vec::new();
        for base in self.org_paths.iter() {
            for entry in WalkDir::new(base).into_iter().filter_map(Result::ok) {
                if !entry.file_type().is_file() {
                    continue;
                }
                if entry.path().extension().and_then(|ext| ext.to_str()) != Some("org") {
                    continue;
                }
                if let Ok(org) = crate::parse::parse_org_file(entry.path()).await {
                    results.push((org, entry.into_path()));
                }
            }
        }
        results
    }
}

pub struct McpServerHandle {
    cancel_token: CancellationToken,
}

impl Drop for McpServerHandle {
    fn drop(&mut self) {
        self.cancel_token.cancel();
    }
}

pub async fn start_mcp_server(
    host: &str,
    port: u16,
    file_resolver: Arc<FileResolver>,
    config: &Config,
) -> AnyResult<McpServerHandle> {
    let bind: SocketAddr = format!("{}:{}", host, port).parse()?;
    let handler = OrgMcpServer::new(file_resolver.clone(), config);

    let sse_config = SseServerConfig {
        bind,
        sse_path: MCP_SSE_PATH.to_string(),
        post_path: MCP_POST_PATH.to_string(),
        ct: CancellationToken::new(),
        sse_keep_alive: None,
    };

    let server = SseServer::serve_with_config(sse_config).await?;
    let cancel_token = server.with_service(move || handler.clone());

    info!(
        address = %bind,
        sse_endpoint = MCP_SSE_PATH,
        post_endpoint = MCP_POST_PATH,
        "MCP server listening"
    );

    Ok(McpServerHandle { cancel_token })
}

#[tool_router]
impl OrgMcpServer {
    #[tool(
        name = "search_org_files",
        description = "Search managed Org files for a query string and return matching lines."
    )]
    async fn search_org_files(
        &self,
        params: Parameters<SearchOrgFilesParams>,
    ) -> Result<Json<SearchOrgFilesResponse>, McpError> {
        let params = params.0;
        let query = params.query.trim();
        if query.is_empty() {
            return Err(McpError::invalid_params("query must not be empty", None));
        }

        let limit = params.limit.unwrap_or(DEFAULT_SEARCH_LIMIT);
        let matches = self
            .collect_search_matches(query, params.case_sensitive, limit)
            .await
            .map_err(|err| McpError::internal_error(err.to_string(), None))?;

        Ok(Json(SearchOrgFilesResponse { matches }))
    }

    #[tool(
        name = "get_org_file_content",
        description = "Return the raw content of an Org file managed by the server."
    )]
    async fn get_org_file_content(
        &self,
        params: Parameters<GetOrgFileParams>,
    ) -> Result<Json<OrgFileContent>, McpError> {
        let params = params.0;
        let path = params.path.trim();
        if path.is_empty() {
            return Err(McpError::invalid_params("path must not be empty", None));
        }

        let resolved = self
            .file_resolver
            .resolve_file(path)
            .await
            .map_err(map_resolver_error)?;

        let content = fs::read_to_string(&resolved)
            .await
            .with_context(|| format!("Failed to read org file: {}", resolved.display()))
            .map_err(|err| McpError::internal_error(err.to_string(), None))?;

        Ok(Json(OrgFileContent {
            path: self.resolve_relative_path(&resolved),
            content,
        }))
    }

    #[tool(
        name = "list_todos",
        description = "List TODO items from managed Org files, optionally filtered by keyword (e.g., TODO, DONE)."
    )]
    async fn list_todos(
        &self,
        params: Parameters<ListTodosParams>,
    ) -> Result<Json<ListTodosResponse>, McpError> {
        let params = params.0;
        let keyword_filter = params
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let mut todos = Vec::new();

        for (org, path) in self.walk_org_files().await {
            self.collect_todos(&org.sections, &mut todos, &path, keyword_filter);
        }

        Ok(Json(ListTodosResponse { todos }))
    }

    #[tool(
        name = "get_agenda",
        description = "Get upcoming SCHEDULED and DEADLINE items from managed Org files."
    )]
    async fn get_agenda(
        &self,
        params: Parameters<GetAgendaParams>,
    ) -> Result<Json<GetAgendaResponse>, McpError> {
        let params = params.0;
        let days = params.days.unwrap_or(7);
        let mut items = Vec::new();
        let now = chrono::Local::now().naive_local();
        let max_date = now + chrono::Duration::days(days as i64);

        for (org, path) in self.walk_org_files().await {
            self.collect_agenda(&org.sections, &mut items, &path, now, max_date);
        }

        Ok(Json(GetAgendaResponse { items }))
    }

    #[tool(
        name = "search_by_tag",
        description = "Search Org files for sections with a specific tag."
    )]
    async fn search_by_tag(
        &self,
        params: Parameters<SearchByTagParams>,
    ) -> Result<Json<SearchByTagResponse>, McpError> {
        let params = params.0;
        let tag = params.tag.trim();
        if tag.is_empty() {
            return Err(McpError::invalid_params("tag must not be empty", None));
        }

        let mut results = Vec::new();

        for (org, path) in self.walk_org_files().await {
            self.collect_by_tag(&org.sections, &mut results, &path, tag);
        }

        Ok(Json(SearchByTagResponse { results }))
    }

    fn collect_todos(
        &self,
        sections: &[org_parser::Section],
        todos: &mut Vec<TodoItem>,
        file_path: &Path,
        keyword_filter: Option<&str>,
    ) {
        for section in sections {
            if let Some(ref status) = section.todo_status
                && keyword_filter.is_none_or(|filter| status.eq_ignore_ascii_case(filter))
            {
                let rel_path = self.resolve_relative_path(file_path);
                todos.push(TodoItem {
                    file: rel_path,
                    line: section.pos.line,
                    headline: section.title.trim().to_string(),
                    keyword: status.clone(),
                });
            }
            self.collect_todos(&section.sections, todos, file_path, keyword_filter);
        }
    }

    fn collect_agenda(
        &self,
        sections: &[org_parser::Section],
        items: &mut Vec<AgendaItem>,
        file_path: &Path,
        now: chrono::NaiveDateTime,
        max_date: chrono::NaiveDateTime,
    ) {
        for section in sections {
            for scheduling in &section.scheduling {
                let (item_type, date_str) = match scheduling {
                    org_parser::Scheduling::Scheduled(_, _, date) => ("SCHEDULED", date),
                    org_parser::Scheduling::Deadline(_, _, date) => ("DEADLINE", date),
                };

                if let Some(parsed_date) = org_parser::reminder::parse_scheduling_datetime(date_str)
                    && parsed_date >= now
                    && parsed_date <= max_date
                {
                    let rel_path = self.resolve_relative_path(file_path);
                    items.push(AgendaItem {
                        file: rel_path,
                        line: section.pos.line,
                        headline: section.title.trim().to_string(),
                        item_type: item_type.to_string(),
                        date: date_str.clone(),
                    });
                }
            }
            self.collect_agenda(&section.sections, items, file_path, now, max_date);
        }
    }

    fn collect_by_tag(
        &self,
        sections: &[org_parser::Section],
        results: &mut Vec<TaggedSection>,
        file_path: &Path,
        tag: &str,
    ) {
        for section in sections {
            if section.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
                let rel_path = self.resolve_relative_path(file_path);
                results.push(TaggedSection {
                    file: rel_path,
                    line: section.pos.line,
                    headline: section.title.trim().to_string(),
                    tags: section.tags.clone(),
                    todo_status: section.todo_status.clone(),
                    priority: section.priority.clone(),
                });
            }
            self.collect_by_tag(&section.sections, results, file_path, tag);
        }
    }
}

#[tool_handler]
impl rmcp::ServerHandler for OrgMcpServer {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo {
            server_info: Implementation {
                name: "org-server-mcp".into(),
                title: Some("Org Server MCP".into()),
                version: env!("CARGO_PKG_VERSION").into(),
                icons: None,
                website_url: None,
            },
            ..Default::default()
        };
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.instructions = Some("Search and inspect Org-mode files managed by org-server.".into());
        info
    }
}

const DEFAULT_SEARCH_LIMIT: usize = 20;
const MCP_SSE_PATH: &str = "/mcp/sse";
const MCP_POST_PATH: &str = "/mcp/message";

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SearchOrgFilesParams {
    pub query: String,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub case_sensitive: bool,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SearchMatch {
    pub file: String,
    pub line: usize,
    pub snippet: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SearchOrgFilesResponse {
    pub matches: Vec<SearchMatch>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GetOrgFileParams {
    pub path: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct OrgFileContent {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ListTodosParams {
    #[serde(default)]
    pub keyword: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct TodoItem {
    pub file: String,
    pub line: usize,
    pub headline: String,
    pub keyword: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ListTodosResponse {
    pub todos: Vec<TodoItem>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GetAgendaParams {
    // Number of days to include in the agenda, starting from today
    #[serde(default)]
    pub days: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AgendaItem {
    pub file: String,
    pub line: usize,
    pub headline: String,
    pub item_type: String, // "SCHEDULED" or "DEADLINE"
    pub date: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GetAgendaResponse {
    pub items: Vec<AgendaItem>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SearchByTagParams {
    pub tag: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct TaggedSection {
    pub file: String,
    pub line: usize,
    pub headline: String,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub todo_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SearchByTagResponse {
    pub results: Vec<TaggedSection>,
}

fn map_resolver_error(err: FileResolverError) -> McpError {
    match err {
        FileResolverError::InvalidPath { message } => McpError::invalid_params(message, None),
        FileResolverError::FileNotFound { filepath } => {
            McpError::invalid_params(format!("Org file not found: {}", filepath), None)
        }
        FileResolverError::InvalidFileExtension => {
            McpError::invalid_params("Only .org files are accessible", None)
        }
        FileResolverError::PathTraversalDetected => {
            McpError::invalid_params("Path traversal detected", None)
        }
        FileResolverError::AbsolutePathNotAllowed => {
            McpError::invalid_params("Absolute paths are not allowed", None)
        }
        FileResolverError::IoError { source } => McpError::internal_error(source.to_string(), None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn build_test_config(base: &Path) -> Config {
        Config {
            org_path: vec![base.display().to_string()],
            server_port: 3000,
            mcp_host: "127.0.0.1".to_string(),
            mcp_port: 3001,
        }
    }

    #[tokio::test]
    async fn search_returns_matches() -> AnyResult<()> {
        let temp_dir = TempDir::new()?;
        let org_path = temp_dir.path().join("notes.org");
        tokio::fs::write(
            &org_path,
            "* TODO Finish report\nSome random line\n* DONE Review report\n",
        )
        .await?;

        let config = build_test_config(temp_dir.path());
        let resolver = Arc::new(FileResolver::new(&config));
        let server = OrgMcpServer::new(resolver, &config);

        let matches = server.collect_search_matches("TODO", false, 10).await?;

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].file, "notes.org");
        assert_eq!(matches[0].line, 1);
        assert!(matches[0].snippet.contains("TODO"));

        Ok(())
    }

    #[tokio::test]
    async fn get_org_file_returns_content() -> AnyResult<()> {
        let temp_dir = TempDir::new()?;
        let org_path = temp_dir.path().join("tasks.org");
        tokio::fs::write(&org_path, "* TODO Write tests\n").await?;

        let config = build_test_config(temp_dir.path());
        let resolver = Arc::new(FileResolver::new(&config));
        let server = OrgMcpServer::new(Arc::clone(&resolver), &config);

        let result = server
            .get_org_file_content(Parameters(GetOrgFileParams {
                path: "tasks.org".to_string(),
            }))
            .await
            .unwrap();

        assert_eq!(result.0.path, "tasks.org");
        assert!(result.0.content.contains("Write tests"));

        Ok(())
    }
}

use crate::config::Config;
use anyhow::Result;
use std::path::PathBuf;
use thiserror::Error;
use tracing::{debug, warn};

/// ファイル解決エラー
#[derive(Error, Debug)]
pub enum FileResolverError {
    #[error("Invalid file path: {message}")]
    InvalidPath { message: String },

    #[error("File not found: {filepath}")]
    FileNotFound { filepath: String },

    #[error("Invalid file extension. Only .org files are allowed")]
    InvalidFileExtension,

    #[error("Path traversal attack detected")]
    PathTraversalDetected,

    #[error("Absolute paths are not allowed")]
    AbsolutePathNotAllowed,

    #[error("IO error: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },
}

/// ファイル解決器
pub struct FileResolver {
    base_paths: Vec<PathBuf>,
}

impl FileResolver {
    /// 新しいファイル解決器を作成
    pub fn new(config: &Config) -> Self {
        let base_paths: Vec<PathBuf> = config.org_path.iter().map(PathBuf::from).collect();

        debug!(
            "FileResolver initialized with {} base paths",
            base_paths.len()
        );
        for (i, path) in base_paths.iter().enumerate() {
            debug!("Base path {}: {}", i + 1, path.display());
        }

        Self { base_paths }
    }

    /// ファイルパスを検証し、正規化する
    pub fn validate_and_normalize_filepath(
        &self,
        filepath: &str,
    ) -> Result<PathBuf, FileResolverError> {
        debug!("Validating filepath: {}", filepath);

        // 1. 空文字列チェック
        if filepath.is_empty() {
            return Err(FileResolverError::InvalidPath {
                message: "Empty filepath".to_string(),
            });
        }

        // 2. パストラバーサル攻撃防止
        if filepath.contains("..") {
            warn!("Path traversal attack detected: {}", filepath);
            return Err(FileResolverError::PathTraversalDetected);
        }

        // 3. 絶対パス禁止
        if filepath.starts_with('/') || filepath.starts_with('\\') {
            warn!("Absolute path not allowed: {}", filepath);
            return Err(FileResolverError::AbsolutePathNotAllowed);
        }

        // 4. .orgファイルのみ許可
        if !filepath.ends_with(".org") {
            warn!("Invalid file extension: {}", filepath);
            return Err(FileResolverError::InvalidFileExtension);
        }

        // 5. パス正規化
        let normalized = PathBuf::from(filepath);

        // 6. 正規化後の追加検証
        if normalized
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            warn!(
                "Parent directory component found after normalization: {}",
                filepath
            );
            return Err(FileResolverError::PathTraversalDetected);
        }

        debug!(
            "Filepath validation successful: {} -> {}",
            filepath,
            normalized.display()
        );
        Ok(normalized)
    }

    /// 指定されたファイルパスを解決し、実際のファイルパスを返す
    pub async fn resolve_file(&self, filepath: &str) -> Result<PathBuf, FileResolverError> {
        let normalized_path = self.validate_and_normalize_filepath(filepath)?;

        debug!("Resolving file: {}", normalized_path.display());

        // 各ベースパスで順次検索
        for (i, base_path) in self.base_paths.iter().enumerate() {
            let full_path = base_path.join(&normalized_path);
            debug!("Checking path {}: {}", i + 1, full_path.display());

            // ファイルの存在確認
            if tokio::fs::metadata(&full_path).await.is_ok() {
                // セキュリティチェック: ベースパス外へのアクセス防止
                let canonical_base = tokio::fs::canonicalize(base_path).await.map_err(|e| {
                    warn!(
                        "Failed to canonicalize base path {}: {}",
                        base_path.display(),
                        e
                    );
                    FileResolverError::IoError { source: e }
                })?;

                let canonical_full = tokio::fs::canonicalize(&full_path).await.map_err(|e| {
                    warn!(
                        "Failed to canonicalize full path {}: {}",
                        full_path.display(),
                        e
                    );
                    FileResolverError::IoError { source: e }
                })?;

                if !canonical_full.starts_with(&canonical_base) {
                    warn!(
                        "Security violation: resolved path {} is outside base path {}",
                        canonical_full.display(),
                        canonical_base.display()
                    );
                    continue;
                }

                debug!("File found: {}", full_path.display());
                return Ok(full_path);
            }
        }

        warn!(
            "File not found in any base path: {}",
            normalized_path.display()
        );
        Err(FileResolverError::FileNotFound {
            filepath: filepath.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use tempfile::TempDir;
    use tokio::fs;

    fn create_test_config(paths: Vec<String>) -> Config {
        Config {
            org_path: paths,
            server_port: 3000,
            server_host: "127.0.0.1".to_string(),
            mcp_host: "127.0.0.1".to_string(),
            mcp_port: 3001,
        }
    }

    #[tokio::test]
    async fn test_validate_filepath_success() {
        let config = create_test_config(vec!["/tmp".to_string()]);
        let resolver = FileResolver::new(&config);

        // 正常なケース
        assert!(resolver.validate_and_normalize_filepath("test.org").is_ok());
        assert!(
            resolver
                .validate_and_normalize_filepath("subdir/test.org")
                .is_ok()
        );
        assert!(
            resolver
                .validate_and_normalize_filepath("deep/nested/path/test.org")
                .is_ok()
        );
    }

    #[tokio::test]
    async fn test_validate_filepath_security_failures() {
        let config = create_test_config(vec!["/tmp".to_string()]);
        let resolver = FileResolver::new(&config);

        // 空文字列
        assert!(matches!(
            resolver.validate_and_normalize_filepath(""),
            Err(FileResolverError::InvalidPath { .. })
        ));

        // パストラバーサル攻撃
        assert!(matches!(
            resolver.validate_and_normalize_filepath("../test.org"),
            Err(FileResolverError::PathTraversalDetected)
        ));

        assert!(matches!(
            resolver.validate_and_normalize_filepath("subdir/../../../test.org"),
            Err(FileResolverError::PathTraversalDetected)
        ));

        // 絶対パス
        assert!(matches!(
            resolver.validate_and_normalize_filepath("/etc/passwd.org"),
            Err(FileResolverError::AbsolutePathNotAllowed)
        ));

        // 無効な拡張子
        assert!(matches!(
            resolver.validate_and_normalize_filepath("test.txt"),
            Err(FileResolverError::InvalidFileExtension)
        ));
    }

    #[tokio::test]
    async fn test_resolve_file_success() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().to_string_lossy().to_string();

        // テストファイルを作成
        let test_file_path = temp_dir.path().join("test.org");
        fs::write(&test_file_path, "* Test content").await?;

        let config = create_test_config(vec![temp_path]);
        let resolver = FileResolver::new(&config);

        // ファイル解決テスト
        let resolved = resolver.resolve_file("test.org").await?;
        assert_eq!(resolved, test_file_path);

        Ok(())
    }

    #[tokio::test]
    async fn test_resolve_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_string_lossy().to_string();

        let config = create_test_config(vec![temp_path]);
        let resolver = FileResolver::new(&config);

        // 存在しないファイル
        assert!(matches!(
            resolver.resolve_file("nonexistent.org").await,
            Err(FileResolverError::FileNotFound { .. })
        ));
    }

    #[tokio::test]
    async fn test_resolve_file_subdirectory() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().to_string_lossy().to_string();

        // サブディレクトリとファイルを作成
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).await?;
        let test_file_path = subdir.join("test.org");
        fs::write(&test_file_path, "* Test content").await?;

        let config = create_test_config(vec![temp_path]);
        let resolver = FileResolver::new(&config);

        // サブディレクトリ内のファイル解決テスト
        let resolved = resolver.resolve_file("subdir/test.org").await?;
        assert_eq!(resolved, test_file_path);

        Ok(())
    }
}

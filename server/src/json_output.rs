use anyhow::{Context as AnyhowContext, Result};
use org_parser::{Context, JsonConversionConfig, JsonConversionError, Org, Section, parse};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use tracing::{debug, info};

/// JSON出力の設定
#[derive(Debug, Clone, Copy)]
pub struct JsonOutputConfig {
    pub pretty: bool,
    pub include_position: bool,
    pub include_empty_sections: bool,
    pub max_depth: usize,
}

impl From<JsonOutputConfig> for JsonConversionConfig {
    fn from(config: JsonOutputConfig) -> Self {
        JsonConversionConfig {
            max_depth: config.max_depth,
            include_position: config.include_position,
            pretty_print: config.pretty,
            include_empty_sections: config.include_empty_sections,
        }
    }
}

/// Orgファイルを解析してJSON形式で出力する
pub fn parse_and_output_json<P: AsRef<Path>, Q: AsRef<Path>>(
    file_path: P,
    output_path: Option<Q>,
    config: JsonOutputConfig,
) -> Result<()> {
    let file_path = file_path.as_ref();

    info!("Parsing Org file: {}", file_path.display());

    // ファイルの存在確認
    if !file_path.exists() {
        anyhow::bail!("File does not exist: {}", file_path.display());
    }

    if !file_path.is_file() {
        anyhow::bail!("Path is not a file: {}", file_path.display());
    }

    // ファイル内容を読み込み
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    debug!("File content length: {} bytes", content.len());

    // Orgファイルを解析
    let mut ctx = Context::new();
    let org = parse(&mut ctx, &content)
        .with_context(|| format!("Failed to parse Org file: {}", file_path.display()))?;

    let total_sections = count_sections_recursive(&org.sections);
    info!(
        "Successfully parsed Org file. Top-level sections: {}, total sections: {}",
        org.sections.len(),
        total_sections
    );

    // JSON変換設定を作成
    let json_config: JsonConversionConfig = config.into();

    // JSONに変換
    let json_output = org.to_json_with_config(&json_config).map_err(|e| match e {
        JsonConversionError::MaxDepthExceeded { depth } => {
            anyhow::anyhow!(
                "Maximum depth exceeded: {}. Consider increasing --max-depth",
                depth
            )
        }
        JsonConversionError::SerializationError { source } => {
            anyhow::anyhow!("JSON serialization failed: {}", source)
        }
        JsonConversionError::IoError { source } => {
            anyhow::anyhow!("IO error: {}", source)
        }
        _ => anyhow::anyhow!("JSON conversion failed: {}", e),
    })?;

    debug!("JSON output length: {} bytes", json_output.len());

    // 出力先に応じて書き込み
    if let Some(output_path) = output_path {
        let output_path = output_path.as_ref();
        write_to_file(output_path, &json_output)
            .with_context(|| format!("Failed to write to file: {}", output_path.display()))?;
        info!("JSON output written to: {}", output_path.display());
    } else {
        write_to_stdout(&json_output).with_context(|| "Failed to write to stdout")?;
        debug!("JSON output written to stdout");
    }

    Ok(())
}

fn count_sections_recursive(sections: &[Section]) -> usize {
    sections
        .iter()
        .map(|section| 1 + count_sections_recursive(&section.sections))
        .sum()
}

/// ファイルにJSON出力を書き込む
fn write_to_file<P: AsRef<Path>>(path: P, content: &str) -> Result<()> {
    let path = path.as_ref();

    // 出力ディレクトリが存在しない場合は作成
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    fs::write(path, content)
        .with_context(|| format!("Failed to write file: {}", path.display()))?;

    Ok(())
}

/// 標準出力にJSON出力を書き込む
fn write_to_stdout(content: &str) -> Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    handle
        .write_all(content.as_bytes())
        .with_context(|| "Failed to write to stdout")?;

    handle
        .write_all(b"\n")
        .with_context(|| "Failed to write newline to stdout")?;

    handle.flush().with_context(|| "Failed to flush stdout")?;

    Ok(())
}

/// 複数のOrgファイルを一括処理する
#[allow(dead_code)]
pub fn parse_multiple_files<P: AsRef<Path>>(
    file_paths: &[P],
    output_dir: Option<&Path>,
    config: JsonOutputConfig,
) -> Result<Vec<Result<(), anyhow::Error>>> {
    let mut results = Vec::new();

    for file_path in file_paths {
        let file_path = file_path.as_ref();

        // 出力ファイル名を決定
        let output_path = match output_dir {
            Some(dir) => {
                let file_stem = file_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("output");
                let output_file = format!("{file_stem}.json");
                Some(dir.join(output_file))
            }
            None => None,
        };

        // 個別ファイルを処理
        let result = parse_and_output_json(file_path, output_path.as_ref(), config);
        results.push(result);
    }

    Ok(results)
}

/// Orgファイルの統計情報を取得
#[allow(dead_code)]
pub fn get_org_stats<P: AsRef<Path>>(file_path: P) -> Result<OrgStats> {
    let file_path = file_path.as_ref();

    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    let mut ctx = Context::new();
    let org = parse(&mut ctx, &content)
        .with_context(|| format!("Failed to parse Org file: {}", file_path.display()))?;

    Ok(OrgStats::from_org(&org))
}

/// Orgファイルの統計情報
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct OrgStats {
    pub title: Option<String>,
    pub total_sections: usize,
    pub max_depth: usize,
    pub total_keywords: usize,
    pub total_properties: usize,
    pub total_drawers: usize,
    pub has_scheduling: bool,
}

impl OrgStats {
    #[allow(dead_code)]
    fn from_org(org: &Org) -> Self {
        let mut stats = OrgStats {
            title: org.title.clone(),
            total_sections: 0,
            max_depth: 0,
            total_keywords: org.keywords.len(),
            total_properties: org.properties.len(),
            total_drawers: org.drawers.len(),
            has_scheduling: false,
        };

        stats.analyze_sections(&org.sections, 1);
        stats
    }

    #[allow(dead_code)]
    fn analyze_sections(&mut self, sections: &[Section], depth: usize) {
        self.total_sections += sections.len();
        self.max_depth = self.max_depth.max(depth);

        for section in sections {
            self.total_keywords += section.keywords.len();
            self.total_properties += section.properties.len();
            self.total_drawers += section.drawers.len();

            if !section.scheduling.is_empty() {
                self.has_scheduling = true;
            }

            if !section.sections.is_empty() {
                self.analyze_sections(&section.sections, depth + 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_org_file(content: &str) -> (TempDir, std::path::PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.org");
        fs::write(&file_path, content).unwrap();
        (temp_dir, file_path)
    }

    #[test]
    fn test_parse_and_output_json_to_stdout() {
        let content = r#"#+TITLE: Test Document

* Section 1
Content for section 1

** Subsection 1.1
Content for subsection 1.1
"#;

        let (_temp_dir, file_path) = create_test_org_file(content);

        let config = JsonOutputConfig {
            pretty: false,
            include_position: false,
            include_empty_sections: true,
            max_depth: 10,
        };

        // 標準出力への出力テスト（実際の出力は確認しないが、エラーが発生しないことを確認）
        let result = parse_and_output_json(&file_path, Option::<&std::path::Path>::None, config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_and_output_json_to_file() {
        let content = r#"#+TITLE: Test Document

* Section 1
Content for section 1
"#;

        let (_temp_dir, file_path) = create_test_org_file(content);
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.json");

        let config = JsonOutputConfig {
            pretty: true,
            include_position: false,
            include_empty_sections: true,
            max_depth: 10,
        };

        let result = parse_and_output_json(&file_path, Some(&output_path), config);
        assert!(result.is_ok());

        // 出力ファイルが作成されたことを確認
        assert!(output_path.exists());

        // 出力内容を確認
        let output_content = fs::read_to_string(&output_path).unwrap();
        assert!(!output_content.is_empty());
        assert!(output_content.contains("Test Document"));
    }

    #[test]
    fn test_parse_nonexistent_file() {
        let config = JsonOutputConfig {
            pretty: false,
            include_position: false,
            include_empty_sections: true,
            max_depth: 10,
        };

        let result = parse_and_output_json("nonexistent.org", None::<&str>, config);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("File does not exist")
        );
    }

    #[test]
    fn test_get_org_stats() {
        let content = r#"#+TITLE: Test Document
#+AUTHOR: Test Author

* Section 1
:PROPERTIES:
:ID: section-1
:END:

Content for section 1

** Subsection 1.1
SCHEDULED: <2024-01-01 Mon>

Content for subsection 1.1

* Section 2
Content for section 2
"#;

        let (_temp_dir, file_path) = create_test_org_file(content);

        let stats = get_org_stats(&file_path).unwrap();

        assert_eq!(stats.title, Some("Test Document".to_string()));
        assert_eq!(stats.total_sections, 3); // Section 1, Subsection 1.1, Section 2
        assert_eq!(stats.max_depth, 2); // depth is 1-indexed from top-level section
        assert!(stats.total_keywords >= 1); // At least TITLE
        assert!(stats.total_properties >= 1); // At least one property
        assert!(stats.has_scheduling); // SCHEDULED keyword present
    }

    #[test]
    fn test_parse_multiple_files() {
        let content1 = "#+TITLE: Document 1\n* Section 1\nContent 1";
        let content2 = "#+TITLE: Document 2\n* Section 2\nContent 2";

        let (_temp_dir1, file_path1) = create_test_org_file(content1);
        let (_temp_dir2, file_path2) = create_test_org_file(content2);

        let output_dir = TempDir::new().unwrap();

        let config = JsonOutputConfig {
            pretty: false,
            include_position: false,
            include_empty_sections: true,
            max_depth: 10,
        };

        let results =
            parse_multiple_files(&[&file_path1, &file_path2], Some(output_dir.path()), config)
                .unwrap();

        // 両方のファイルが正常に処理されたことを確認
        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());

        // 出力ファイルが作成されたことを確認
        assert!(output_dir.path().join("test.json").exists());
    }
}

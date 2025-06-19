use crate::json_conversion::{JsonConversionConfig, JsonConversionError};
use crate::parser::{Context, Org, parse};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;

/// テスト用のヘルパー関数とユーティリティ
///
/// テストリソースファイルを読み込む
pub fn load_test_resource(filename: &str) -> Result<String, std::io::Error> {
    let path = format!("tests/resources/{}", filename);
    fs::read_to_string(&path)
}

/// Orgファイルをパースしてテスト用のOrg構造体を返す
pub fn parse_test_org(content: &str) -> Result<Org, Box<dyn std::error::Error>> {
    let mut ctx = Context::new();
    parse(&mut ctx, content).map_err(|e| e.into())
}

/// テストリソースファイルをパースしてOrg構造体を返す
pub fn parse_test_resource(filename: &str) -> Result<Org, Box<dyn std::error::Error>> {
    let content = load_test_resource(filename)?;
    parse_test_org(&content)
}

/// 往復変換テスト（org → json → org）
pub fn test_roundtrip_conversion(
    org: &Org,
    config: &JsonConversionConfig,
) -> Result<RoundtripResult, JsonConversionError> {
    // org → json
    let json = org.to_json_with_config(config)?;

    // json → org
    let restored_org = Org::from_json(&json)?;

    // 再度json変換して比較
    let restored_json = restored_org.to_json_with_config(config)?;

    Ok(RoundtripResult {
        original_org: org.clone(),
        restored_org,
        original_json: json,
        restored_json,
    })
}

/// 往復変換の結果
#[derive(Debug)]
pub struct RoundtripResult {
    pub original_org: Org,
    pub restored_org: Org,
    pub original_json: String,
    pub restored_json: String,
}

impl RoundtripResult {
    /// 基本的な構造の一致を確認
    pub fn verify_basic_structure(&self) -> Vec<String> {
        let mut errors = Vec::new();

        // タイトルの比較
        if self.original_org.title != self.restored_org.title {
            errors.push(format!(
                "Title mismatch: {:?} != {:?}",
                self.original_org.title, self.restored_org.title
            ));
        }

        // セクション数の比較
        if self.original_org.sections.len() != self.restored_org.sections.len() {
            errors.push(format!(
                "Section count mismatch: {} != {}",
                self.original_org.sections.len(),
                self.restored_org.sections.len()
            ));
        }

        // キーワード数の比較
        if self.original_org.keywords.len() != self.restored_org.keywords.len() {
            errors.push(format!(
                "Keyword count mismatch: {} != {}",
                self.original_org.keywords.len(),
                self.restored_org.keywords.len()
            ));
        }

        // プロパティ数の比較
        if self.original_org.properties.len() != self.restored_org.properties.len() {
            errors.push(format!(
                "Properties count mismatch: {} != {}",
                self.original_org.properties.len(),
                self.restored_org.properties.len()
            ));
        }

        errors
    }

    /// セクションの詳細比較
    pub fn verify_sections(&self) -> Vec<String> {
        let mut errors = Vec::new();

        for (i, (orig, restored)) in self
            .original_org
            .sections
            .iter()
            .zip(self.restored_org.sections.iter())
            .enumerate()
        {
            if orig.title != restored.title {
                errors.push(format!(
                    "Section[{}] title mismatch: {:?} != {:?}",
                    i, orig.title, restored.title
                ));
            }

            if orig.headline_symbol != restored.headline_symbol {
                errors.push(format!(
                    "Section[{}] headline_symbol mismatch: {:?} != {:?}",
                    i, orig.headline_symbol, restored.headline_symbol
                ));
            }

            if orig.contents.len() != restored.contents.len() {
                errors.push(format!(
                    "Section[{}] content count mismatch: {} != {}",
                    i,
                    orig.contents.len(),
                    restored.contents.len()
                ));
            }
        }

        errors
    }

    /// JSON構造の比較
    pub fn verify_json_structure(&self) -> Result<Vec<String>, serde_json::Error> {
        let mut errors = Vec::new();

        let original_value: Value = serde_json::from_str(&self.original_json)?;
        let restored_value: Value = serde_json::from_str(&self.restored_json)?;

        if original_value != restored_value {
            errors.push("JSON structures are not identical".to_string());

            // より詳細な比較
            if let (Value::Object(orig_obj), Value::Object(rest_obj)) =
                (&original_value, &restored_value)
            {
                for key in orig_obj.keys() {
                    if !rest_obj.contains_key(key) {
                        errors.push(format!("Missing key in restored JSON: {}", key));
                    }
                }

                for key in rest_obj.keys() {
                    if !orig_obj.contains_key(key) {
                        errors.push(format!("Extra key in restored JSON: {}", key));
                    }
                }
            }
        }

        Ok(errors)
    }

    /// 完全な検証を実行
    pub fn verify_complete(&self) -> Result<ValidationReport, serde_json::Error> {
        let mut report = ValidationReport::new();

        report.basic_structure_errors = self.verify_basic_structure();
        report.section_errors = self.verify_sections();
        report.json_structure_errors = self.verify_json_structure()?;

        Ok(report)
    }
}

/// 検証レポート
#[derive(Debug)]
pub struct ValidationReport {
    pub basic_structure_errors: Vec<String>,
    pub section_errors: Vec<String>,
    pub json_structure_errors: Vec<String>,
}

impl Default for ValidationReport {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationReport {
    pub fn new() -> Self {
        Self {
            basic_structure_errors: Vec::new(),
            section_errors: Vec::new(),
            json_structure_errors: Vec::new(),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.basic_structure_errors.is_empty()
            && self.section_errors.is_empty()
            && self.json_structure_errors.is_empty()
    }

    pub fn total_errors(&self) -> usize {
        self.basic_structure_errors.len()
            + self.section_errors.len()
            + self.json_structure_errors.len()
    }

    pub fn print_summary(&self) {
        println!("Validation Report:");
        println!(
            "  Basic structure errors: {}",
            self.basic_structure_errors.len()
        );
        println!("  Section errors: {}", self.section_errors.len());
        println!(
            "  JSON structure errors: {}",
            self.json_structure_errors.len()
        );
        println!("  Total errors: {}", self.total_errors());

        if !self.is_valid() {
            println!("\nErrors:");
            for error in &self.basic_structure_errors {
                println!("  [BASIC] {}", error);
            }
            for error in &self.section_errors {
                println!("  [SECTION] {}", error);
            }
            for error in &self.json_structure_errors {
                println!("  [JSON] {}", error);
            }
        }
    }
}

/// JSON構造の統計情報を収集
pub fn analyze_json_structure(json: &str) -> Result<JsonAnalysis, serde_json::Error> {
    let value: Value = serde_json::from_str(json)?;
    let mut analysis = JsonAnalysis::new();
    analyze_value(&value, &mut analysis, 0);
    Ok(analysis)
}

/// JSON構造の分析結果
#[derive(Debug)]
pub struct JsonAnalysis {
    pub total_objects: usize,
    pub total_arrays: usize,
    pub total_strings: usize,
    pub total_numbers: usize,
    pub total_booleans: usize,
    pub total_nulls: usize,
    pub max_depth: usize,
    pub unique_keys: HashMap<String, usize>,
}

impl Default for JsonAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonAnalysis {
    pub fn new() -> Self {
        Self {
            total_objects: 0,
            total_arrays: 0,
            total_strings: 0,
            total_numbers: 0,
            total_booleans: 0,
            total_nulls: 0,
            max_depth: 0,
            unique_keys: HashMap::new(),
        }
    }

    pub fn print_summary(&self) {
        println!("JSON Analysis:");
        println!("  Objects: {}", self.total_objects);
        println!("  Arrays: {}", self.total_arrays);
        println!("  Strings: {}", self.total_strings);
        println!("  Numbers: {}", self.total_numbers);
        println!("  Booleans: {}", self.total_booleans);
        println!("  Nulls: {}", self.total_nulls);
        println!("  Max depth: {}", self.max_depth);
        println!("  Unique keys: {}", self.unique_keys.len());
    }
}

fn analyze_value(value: &Value, analysis: &mut JsonAnalysis, depth: usize) {
    analysis.max_depth = analysis.max_depth.max(depth);

    match value {
        Value::Object(obj) => {
            analysis.total_objects += 1;
            for (key, val) in obj {
                *analysis.unique_keys.entry(key.clone()).or_insert(0) += 1;
                analyze_value(val, analysis, depth + 1);
            }
        }
        Value::Array(arr) => {
            analysis.total_arrays += 1;
            for val in arr {
                analyze_value(val, analysis, depth + 1);
            }
        }
        Value::String(_) => analysis.total_strings += 1,
        Value::Number(_) => analysis.total_numbers += 1,
        Value::Bool(_) => analysis.total_booleans += 1,
        Value::Null => analysis.total_nulls += 1,
    }
}

/// パフォーマンステスト用のヘルパー
pub struct PerformanceTest {
    pub name: String,
    pub start_time: std::time::Instant,
}

impl PerformanceTest {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            start_time: std::time::Instant::now(),
        }
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    pub fn finish(self) -> std::time::Duration {
        let duration = self.elapsed();
        println!(
            "Performance test '{}' completed in {:?}",
            self.name, duration
        );
        duration
    }
}

/// メモリ使用量の測定（概算）
pub fn estimate_memory_usage(org: &Org) -> usize {
    let mut total_size = std::mem::size_of::<Org>();

    // タイトル、ID、ファイル名のサイズ
    if let Some(title) = &org.title {
        total_size += title.len();
    }
    if let Some(id) = &org.id {
        total_size += id.len();
    }
    if let Some(filename) = &org.filename {
        total_size += filename.len();
    }

    // セクション数に基づく概算
    total_size += org.sections.len() * 1000; // セクションあたり約1KB
    total_size += org.keywords.len() * 100; // キーワードあたり約100B
    total_size += org.properties.len() * 200; // プロパティあたり約200B

    total_size
}

/// テスト用のOrg構造体生成器
pub struct OrgGenerator {
    section_count: usize,
    max_depth: usize,
    content_per_section: usize,
}

impl OrgGenerator {
    pub fn new() -> Self {
        Self {
            section_count: 10,
            max_depth: 3,
            content_per_section: 5,
        }
    }

    pub fn with_sections(mut self, count: usize) -> Self {
        self.section_count = count;
        self
    }

    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    pub fn with_content_per_section(mut self, count: usize) -> Self {
        self.content_per_section = count;
        self
    }

    pub fn generate(&self) -> String {
        let mut content = String::new();
        content.push_str("#+TITLE: Generated Test Document\n\n");

        for i in 0..self.section_count {
            self.generate_section(&mut content, i, 1);
        }

        content
    }

    fn generate_section(&self, content: &mut String, index: usize, depth: usize) {
        if depth > self.max_depth {
            return;
        }

        // セクションヘッダー
        content.push_str(&"*".repeat(depth));
        content.push_str(&format!(" Section {} at depth {}\n", index, depth));

        // プロパティ
        content.push_str(":PROPERTIES:\n");
        content.push_str(&format!(":ID:       section-{}-{}\n", depth, index));
        content.push_str(":CATEGORY: test\n");
        content.push_str(":END:\n\n");

        // コンテンツ
        for j in 0..self.content_per_section {
            content.push_str(&format!(
                "Content line {} for section {}-{}\n",
                j, depth, index
            ));
        }
        content.push('\n');

        // サブセクション
        if depth < self.max_depth {
            for k in 0..2 {
                self.generate_section(content, k, depth + 1);
            }
        }
    }
}

impl Default for OrgGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_org_generator() {
        let generator = OrgGenerator::new()
            .with_sections(3)
            .with_max_depth(2)
            .with_content_per_section(2);

        let content = generator.generate();
        assert!(!content.is_empty());
        assert!(content.contains("#+TITLE: Generated Test Document"));
        assert!(content.contains("Section 0 at depth 1"));
    }

    #[test]
    fn test_json_analysis() {
        let json = r#"{"title": "test", "sections": [{"id": "1", "title": "section1"}]}"#;
        let analysis = analyze_json_structure(json).expect("Failed to analyze JSON");

        assert!(analysis.total_objects > 0);
        assert!(analysis.total_strings > 0);
        assert!(analysis.unique_keys.contains_key("title"));
    }

    #[test]
    fn test_performance_test() {
        let test = PerformanceTest::new("test");
        std::thread::sleep(std::time::Duration::from_millis(10));
        let duration = test.finish();
        assert!(duration.as_millis() >= 10);
    }
}

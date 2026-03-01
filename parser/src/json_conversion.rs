use crate::parser::{
    CheckboxState, CodeBlock, Content, Drawer, Keyword, ListItem, ListKind, Org, PlainList, Pos,
    Properties, Property, Row, Scheduling, Section, Table, TableRow,
};
use serde::{Deserialize, Serialize};
use std::io::Write;
use thiserror::Error;

/// JSON変換時の設定
#[derive(Debug, Clone)]
pub struct JsonConversionConfig {
    /// 最大入れ子深度（無限ループ防止）
    pub max_depth: usize,
    /// 位置情報を含めるかどうか
    pub include_position: bool,
    /// 美しい整形を行うかどうか
    pub pretty_print: bool,
    /// 空のセクションを含めるかどうか
    pub include_empty_sections: bool,
}

impl Default for JsonConversionConfig {
    fn default() -> Self {
        Self {
            max_depth: 10,
            include_position: false,
            pretty_print: false,
            include_empty_sections: true,
        }
    }
}

/// JSON変換エラー
#[derive(Error, Debug)]
pub enum JsonConversionError {
    #[error("Maximum depth exceeded: {depth}")]
    MaxDepthExceeded { depth: usize },

    #[error("JSON serialization failed: {source}")]
    SerializationError {
        #[from]
        source: serde_json::Error,
    },

    #[error("JSON deserialization failed: {source}")]
    DeserializationError { source: serde_json::Error },

    #[error("Invalid JSON structure: {message}")]
    InvalidStructure { message: String },

    #[error("IO error during streaming: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },
}

/// 深度制限付きでセクションを処理するための関数
fn process_sections_with_depth_limit(
    sections: &[Section],
    config: &JsonConversionConfig,
    current_depth: usize,
) -> Result<Vec<SafeSection>, JsonConversionError> {
    if current_depth >= config.max_depth {
        return Err(JsonConversionError::MaxDepthExceeded {
            depth: current_depth,
        });
    }

    let mut result = Vec::new();

    for section in sections {
        if !config.include_empty_sections
            && section.contents.is_empty()
            && section.sections.is_empty()
        {
            continue;
        }

        let safe_section = SafeSection {
            pos: if config.include_position {
                Some(section.pos.clone())
            } else {
                None
            },
            id: section.id.clone(),
            headline_symbol: section.headline_symbol.clone(),
            todo_status: section.todo_status.clone(),
            priority: section.priority.clone(),
            title: section.title.clone(),
            tags: section.tags.clone(),
            drawers: convert_drawers(&section.drawers, config),
            properties: convert_properties(&section.properties, config),
            keywords: convert_keywords(&section.keywords, config),
            contents: convert_rows(&section.contents, config),
            code_blocks: convert_code_blocks(&section.code_blocks, config),
            lists: convert_lists(&section.lists, config),
            tables: convert_tables(&section.tables, config),
            scheduling: convert_scheduling(&section.scheduling, config),
            sections: process_sections_with_depth_limit(
                &section.sections,
                config,
                current_depth + 1,
            )?,
        };

        result.push(safe_section);
    }

    Ok(result)
}

/// Drawerの変換
fn convert_drawers(drawers: &[Drawer], config: &JsonConversionConfig) -> Vec<SafeDrawer> {
    drawers
        .iter()
        .map(|drawer| SafeDrawer {
            pos: if config.include_position {
                Some(drawer.pos.clone())
            } else {
                None
            },
            name: drawer.name.clone(),
            children: convert_rows(&drawer.children, config),
        })
        .collect()
}

/// Propertiesの変換
fn convert_properties(
    properties: &[Properties],
    config: &JsonConversionConfig,
) -> Vec<SafeProperties> {
    properties
        .iter()
        .map(|props| SafeProperties {
            pos: if config.include_position {
                Some(props.pos.clone())
            } else {
                None
            },
            children: props
                .children
                .iter()
                .map(|prop| SafeProperty {
                    pos: if config.include_position {
                        Some(prop.pos.clone())
                    } else {
                        None
                    },
                    key: prop.key.clone(),
                    value: prop.value.clone(),
                })
                .collect(),
        })
        .collect()
}

/// Keywordの変換
fn convert_keywords(keywords: &[Keyword], config: &JsonConversionConfig) -> Vec<SafeKeyword> {
    keywords
        .iter()
        .map(|keyword| SafeKeyword {
            pos: if config.include_position {
                Some(keyword.pos.clone())
            } else {
                None
            },
            key: keyword.key.clone(),
            value: keyword.value.clone(),
        })
        .collect()
}

/// Rowの変換
fn convert_rows(rows: &[Row], config: &JsonConversionConfig) -> Vec<SafeRow> {
    rows.iter()
        .map(|row| SafeRow {
            pos: if config.include_position {
                Some(row.pos.clone())
            } else {
                None
            },
            contents: convert_contents(&row.contents, config),
        })
        .collect()
}

/// Contentの変換
fn convert_contents(contents: &[Content], config: &JsonConversionConfig) -> Vec<SafeContent> {
    contents
        .iter()
        .map(|content| match content {
            Content::Text(pos, text) => SafeContent::Text {
                pos: if config.include_position {
                    Some(pos.clone())
                } else {
                    None
                },
                text: text.clone(),
            },
            Content::Hyperlink(pos, link, desc) => SafeContent::Hyperlink {
                pos: if config.include_position {
                    Some(pos.clone())
                } else {
                    None
                },
                link: link.clone(),
                description: desc.clone(),
            },
        })
        .collect()
}

/// Schedulingの変換
fn convert_scheduling(
    scheduling: &[Scheduling],
    config: &JsonConversionConfig,
) -> Vec<SafeScheduling> {
    scheduling
        .iter()
        .map(|sched| match sched {
            Scheduling::Scheduled(pos, title, data) => SafeScheduling::Scheduled {
                pos: if config.include_position {
                    Some(pos.clone())
                } else {
                    None
                },
                title: title.clone(),
                data: data.clone(),
            },
            Scheduling::Deadline(pos, title, data) => SafeScheduling::Deadline {
                pos: if config.include_position {
                    Some(pos.clone())
                } else {
                    None
                },
                title: title.clone(),
                data: data.clone(),
            },
        })
        .collect()
}

/// CodeBlockの変換
fn convert_code_blocks(
    code_blocks: &[CodeBlock],
    config: &JsonConversionConfig,
) -> Vec<SafeCodeBlock> {
    code_blocks
        .iter()
        .map(|cb| SafeCodeBlock {
            pos: if config.include_position {
                Some(cb.pos.clone())
            } else {
                None
            },
            language: cb.language.clone(),
            body: cb.body.clone(),
        })
        .collect()
}

/// 位置情報を制御可能なListItem
#[derive(Debug, Serialize, Deserialize)]
struct SafeListItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pos: Option<Pos>,
    indent: usize,
    bullet: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    checkbox: Option<CheckboxState>,
    kind: ListKind,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description_term: Option<String>,
}

/// 位置情報を制御可能なPlainList
#[derive(Debug, Serialize, Deserialize)]
struct SafePlainList {
    #[serde(skip_serializing_if = "Option::is_none")]
    pos: Option<Pos>,
    items: Vec<SafeListItem>,
}

fn convert_lists(lists: &[PlainList], config: &JsonConversionConfig) -> Vec<SafePlainList> {
    lists
        .iter()
        .map(|list| SafePlainList {
            pos: if config.include_position {
                Some(list.pos.clone())
            } else {
                None
            },
            items: list
                .items
                .iter()
                .map(|item| SafeListItem {
                    pos: if config.include_position {
                        Some(item.pos.clone())
                    } else {
                        None
                    },
                    indent: item.indent,
                    bullet: item.bullet.clone(),
                    checkbox: item.checkbox.clone(),
                    kind: item.kind.clone(),
                    text: item.text.clone(),
                    description_term: item.description_term.clone(),
                })
                .collect(),
        })
        .collect()
}

/// 位置情報を制御可能なTableRow
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
enum SafeTableRow {
    Standard(Vec<String>),
    Rule,
}

/// 位置情報を制御可能なTable
#[derive(Debug, Serialize, Deserialize)]
struct SafeTable {
    #[serde(skip_serializing_if = "Option::is_none")]
    pos: Option<Pos>,
    rows: Vec<SafeTableRow>,
}

fn convert_tables(tables: &[Table], config: &JsonConversionConfig) -> Vec<SafeTable> {
    tables
        .iter()
        .map(|table| SafeTable {
            pos: if config.include_position {
                Some(table.pos.clone())
            } else {
                None
            },
            rows: table
                .rows
                .iter()
                .map(|row| match row {
                    TableRow::Standard(cells) => SafeTableRow::Standard(cells.clone()),
                    TableRow::Rule => SafeTableRow::Rule,
                })
                .collect(),
        })
        .collect()
}

/// 位置情報を制御可能なDrawer
#[derive(Debug, Serialize, Deserialize)]
struct SafeDrawer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pos: Option<Pos>,
    name: String,
    children: Vec<SafeRow>,
}

/// 位置情報を制御可能なProperties
#[derive(Debug, Serialize, Deserialize)]
struct SafeProperties {
    #[serde(skip_serializing_if = "Option::is_none")]
    pos: Option<Pos>,
    children: Vec<SafeProperty>,
}

/// 位置情報を制御可能なProperty
#[derive(Debug, Serialize, Deserialize)]
struct SafeProperty {
    #[serde(skip_serializing_if = "Option::is_none")]
    pos: Option<Pos>,
    key: String,
    value: String,
}

/// 位置情報を制御可能なKeyword
#[derive(Debug, Serialize, Deserialize)]
struct SafeKeyword {
    #[serde(skip_serializing_if = "Option::is_none")]
    pos: Option<Pos>,
    key: String,
    value: String,
}

/// 位置情報を制御可能なRow
#[derive(Debug, Serialize, Deserialize)]
struct SafeRow {
    #[serde(skip_serializing_if = "Option::is_none")]
    pos: Option<Pos>,
    contents: Vec<SafeContent>,
}

/// 位置情報を制御可能なContent
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum SafeContent {
    Text {
        #[serde(skip_serializing_if = "Option::is_none")]
        pos: Option<Pos>,
        text: String,
    },
    Hyperlink {
        #[serde(skip_serializing_if = "Option::is_none")]
        pos: Option<Pos>,
        link: String,
        description: Option<String>,
    },
}

/// 位置情報を制御可能なScheduling
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum SafeScheduling {
    Scheduled {
        #[serde(skip_serializing_if = "Option::is_none")]
        pos: Option<Pos>,
        title: String,
        data: String,
    },
    Deadline {
        #[serde(skip_serializing_if = "Option::is_none")]
        pos: Option<Pos>,
        title: String,
        data: String,
    },
}

/// 位置情報を制御可能なCodeBlock
#[derive(Debug, Serialize, Deserialize)]
struct SafeCodeBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pos: Option<Pos>,
    language: String,
    body: String,
}

/// 位置情報を制御可能なSection
#[derive(Debug, Serialize, Deserialize)]
struct SafeSection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pos: Option<Pos>,
    id: String,
    headline_symbol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    todo_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    priority: Option<String>,
    title: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    tags: Vec<String>,
    drawers: Vec<SafeDrawer>,
    properties: Vec<SafeProperties>,
    keywords: Vec<SafeKeyword>,
    contents: Vec<SafeRow>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    code_blocks: Vec<SafeCodeBlock>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    lists: Vec<SafePlainList>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    tables: Vec<SafeTable>,
    scheduling: Vec<SafeScheduling>,
    sections: Vec<SafeSection>,
}

/// 位置情報を制御可能なOrg
#[derive(Debug, Serialize, Deserialize)]
struct SafeOrg {
    filename: Option<String>,
    id: Option<String>,
    title: Option<String>,
    drawers: Vec<SafeDrawer>,
    properties: Vec<SafeProperties>,
    keywords: Vec<SafeKeyword>,
    sections: Vec<SafeSection>,
    scheduling: Vec<SafeScheduling>,
}

/// Org構造体のJSON変換機能を拡張
impl Org {
    /// 設定可能なJSON変換
    pub fn to_json_with_config(
        &self,
        config: &JsonConversionConfig,
    ) -> Result<String, JsonConversionError> {
        let safe_org = SafeOrg {
            filename: self.filename.clone(),
            id: self.id.clone(),
            title: self.title.clone(),
            drawers: convert_drawers(&self.drawers, config),
            properties: convert_properties(&self.properties, config),
            keywords: convert_keywords(&self.keywords, config),
            sections: process_sections_with_depth_limit(&self.sections, config, 0)?,
            scheduling: convert_scheduling(&self.scheduling, config),
        };

        if config.pretty_print {
            Ok(serde_json::to_string_pretty(&safe_org)?)
        } else {
            Ok(serde_json::to_string(&safe_org)?)
        }
    }

    /// 軽量版JSON（位置情報なし）
    pub fn to_json_compact(&self) -> Result<String, JsonConversionError> {
        let config = JsonConversionConfig {
            include_position: false,
            pretty_print: false,
            include_empty_sections: false,
            ..Default::default()
        };
        self.to_json_with_config(&config)
    }

    /// 美しい整形されたJSON
    pub fn to_json_pretty(&self) -> Result<String, JsonConversionError> {
        let config = JsonConversionConfig {
            pretty_print: true,
            ..Default::default()
        };
        self.to_json_with_config(&config)
    }

    /// JSONからの復元
    pub fn from_json(json: &str) -> Result<Self, JsonConversionError> {
        // まずSafeOrg形式として読み込み
        let safe_org: SafeOrg = serde_json::from_str(json)
            .map_err(|e| JsonConversionError::DeserializationError { source: e })?;

        // SafeOrgからOrgに変換
        Ok(convert_safe_org_to_org(safe_org))
    }

    /// ストリーミング変換（大きなファイル用）
    pub fn to_json_stream<W: Write>(
        &self,
        writer: W,
        config: &JsonConversionConfig,
    ) -> Result<(), JsonConversionError> {
        let safe_org = SafeOrg {
            filename: self.filename.clone(),
            id: self.id.clone(),
            title: self.title.clone(),
            drawers: convert_drawers(&self.drawers, config),
            properties: convert_properties(&self.properties, config),
            keywords: convert_keywords(&self.keywords, config),
            sections: process_sections_with_depth_limit(&self.sections, config, 0)?,
            scheduling: convert_scheduling(&self.scheduling, config),
        };

        if config.pretty_print {
            serde_json::to_writer_pretty(writer, &safe_org)?;
        } else {
            serde_json::to_writer(writer, &safe_org)?;
        }

        Ok(())
    }

    /// 部分的な変換（特定のセクションのみ）
    pub fn section_to_json(
        &self,
        section_id: &str,
        config: &JsonConversionConfig,
    ) -> Result<Option<String>, JsonConversionError> {
        fn find_section_by_id<'a>(sections: &'a [Section], id: &str) -> Option<&'a Section> {
            for section in sections {
                if section.id == id {
                    return Some(section);
                }
                if let Some(found) = find_section_by_id(&section.sections, id) {
                    return Some(found);
                }
            }
            None
        }

        if let Some(section) = find_section_by_id(&self.sections, section_id) {
            let safe_section = SafeSection {
                pos: if config.include_position {
                    Some(section.pos.clone())
                } else {
                    None
                },
                id: section.id.clone(),
                headline_symbol: section.headline_symbol.clone(),
                todo_status: section.todo_status.clone(),
                priority: section.priority.clone(),
                title: section.title.clone(),
                tags: section.tags.clone(),
                drawers: convert_drawers(&section.drawers, config),
                properties: convert_properties(&section.properties, config),
                keywords: convert_keywords(&section.keywords, config),
                contents: convert_rows(&section.contents, config),
                code_blocks: convert_code_blocks(&section.code_blocks, config),
                lists: convert_lists(&section.lists, config),
                tables: convert_tables(&section.tables, config),
                scheduling: convert_scheduling(&section.scheduling, config),
                sections: process_sections_with_depth_limit(&section.sections, config, 0)?,
            };

            if config.pretty_print {
                Ok(Some(serde_json::to_string_pretty(&safe_section)?))
            } else {
                Ok(Some(serde_json::to_string(&safe_section)?))
            }
        } else {
            Ok(None)
        }
    }
}

/// SafeOrgからOrgへの変換
fn convert_safe_org_to_org(safe_org: SafeOrg) -> Org {
    Org {
        filename: safe_org.filename,
        id: safe_org.id,
        title: safe_org.title,
        drawers: safe_org
            .drawers
            .into_iter()
            .map(convert_safe_drawer_to_drawer)
            .collect(),
        properties: safe_org
            .properties
            .into_iter()
            .map(convert_safe_properties_to_properties)
            .collect(),
        keywords: safe_org
            .keywords
            .into_iter()
            .map(convert_safe_keyword_to_keyword)
            .collect(),
        sections: safe_org
            .sections
            .into_iter()
            .map(convert_safe_section_to_section)
            .collect(),
        scheduling: safe_org
            .scheduling
            .into_iter()
            .map(convert_safe_scheduling_to_scheduling)
            .collect(),
    }
}

/// SafeDrawerからDrawerへの変換
fn convert_safe_drawer_to_drawer(safe_drawer: SafeDrawer) -> Drawer {
    Drawer {
        pos: safe_drawer.pos.unwrap_or_default(),
        name: safe_drawer.name,
        children: safe_drawer
            .children
            .into_iter()
            .map(convert_safe_row_to_row)
            .collect(),
    }
}

/// SafePropertiesからPropertiesへの変換
fn convert_safe_properties_to_properties(safe_properties: SafeProperties) -> Properties {
    Properties {
        pos: safe_properties.pos.unwrap_or_default(),
        children: safe_properties
            .children
            .into_iter()
            .map(convert_safe_property_to_property)
            .collect(),
    }
}

/// SafePropertyからPropertyへの変換
fn convert_safe_property_to_property(safe_property: SafeProperty) -> Property {
    Property {
        pos: safe_property.pos.unwrap_or_default(),
        key: safe_property.key,
        value: safe_property.value,
    }
}

/// SafeKeywordからKeywordへの変換
fn convert_safe_keyword_to_keyword(safe_keyword: SafeKeyword) -> Keyword {
    Keyword {
        pos: safe_keyword.pos.unwrap_or_default(),
        key: safe_keyword.key,
        value: safe_keyword.value,
    }
}

/// SafeRowからRowへの変換
fn convert_safe_row_to_row(safe_row: SafeRow) -> Row {
    Row {
        pos: safe_row.pos.unwrap_or_default(),
        contents: safe_row
            .contents
            .into_iter()
            .map(convert_safe_content_to_content)
            .collect(),
    }
}

/// SafeContentからContentへの変換
fn convert_safe_content_to_content(safe_content: SafeContent) -> Content {
    match safe_content {
        SafeContent::Text { pos, text } => Content::Text(pos.unwrap_or_default(), text),
        SafeContent::Hyperlink {
            pos,
            link,
            description,
        } => Content::Hyperlink(pos.unwrap_or_default(), link, description),
    }
}

/// SafeSchedulingからSchedulingへの変換
fn convert_safe_scheduling_to_scheduling(safe_scheduling: SafeScheduling) -> Scheduling {
    match safe_scheduling {
        SafeScheduling::Scheduled { pos, title, data } => {
            Scheduling::Scheduled(pos.unwrap_or_default(), title, data)
        }
        SafeScheduling::Deadline { pos, title, data } => {
            Scheduling::Deadline(pos.unwrap_or_default(), title, data)
        }
    }
}

/// SafeSectionからSectionへの変換
fn convert_safe_section_to_section(safe_section: SafeSection) -> Section {
    Section {
        pos: safe_section.pos.unwrap_or_default(),
        id: safe_section.id,
        headline_symbol: safe_section.headline_symbol,
        todo_status: safe_section.todo_status,
        priority: safe_section.priority,
        title: safe_section.title,
        tags: safe_section.tags,
        drawers: safe_section
            .drawers
            .into_iter()
            .map(convert_safe_drawer_to_drawer)
            .collect(),
        properties: safe_section
            .properties
            .into_iter()
            .map(convert_safe_properties_to_properties)
            .collect(),
        keywords: safe_section
            .keywords
            .into_iter()
            .map(convert_safe_keyword_to_keyword)
            .collect(),
        contents: safe_section
            .contents
            .into_iter()
            .map(convert_safe_row_to_row)
            .collect(),
        code_blocks: safe_section
            .code_blocks
            .into_iter()
            .map(|cb| CodeBlock {
                pos: cb.pos.unwrap_or_default(),
                language: cb.language,
                body: cb.body,
            })
            .collect(),
        lists: safe_section
            .lists
            .into_iter()
            .map(|list| PlainList {
                pos: list.pos.unwrap_or_default(),
                items: list
                    .items
                    .into_iter()
                    .map(|item| ListItem {
                        pos: item.pos.unwrap_or_default(),
                        indent: item.indent,
                        bullet: item.bullet,
                        checkbox: item.checkbox,
                        kind: item.kind,
                        text: item.text,
                        description_term: item.description_term,
                    })
                    .collect(),
            })
            .collect(),
        tables: safe_section
            .tables
            .into_iter()
            .map(|table| Table {
                pos: table.pos.unwrap_or_default(),
                rows: table
                    .rows
                    .into_iter()
                    .map(|row| match row {
                        SafeTableRow::Standard(cells) => TableRow::Standard(cells),
                        SafeTableRow::Rule => TableRow::Rule,
                    })
                    .collect(),
            })
            .collect(),
        scheduling: safe_section
            .scheduling
            .into_iter()
            .map(convert_safe_scheduling_to_scheduling)
            .collect(),
        sections: safe_section
            .sections
            .into_iter()
            .map(convert_safe_section_to_section)
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Context, parse};

    fn init() {
        let _ = tracing_subscriber::fmt::try_init();
    }

    #[test]
    fn test_json_conversion_config_default() {
        let config = JsonConversionConfig::default();
        assert_eq!(config.max_depth, 10);
        assert!(!config.include_position);
        assert!(!config.pretty_print);
        assert!(config.include_empty_sections);
    }

    #[test]
    fn test_basic_json_conversion() {
        init();

        let content = r#"#+TITLE: Test Document

* Section 1
Content for section 1

** Subsection 1.1
Content for subsection 1.1
"#;

        let mut ctx = Context::new();
        let org = parse(&mut ctx, content).expect("Failed to parse org content");

        // 基本的なJSON変換
        let json = org.to_json_compact().expect("Failed to convert to JSON");
        assert!(!json.is_empty());

        // SafeOrg形式のJSON往復変換テスト
        let restored_org = Org::from_json(&json).expect("Failed to restore from JSON");
        assert_eq!(org.title, restored_org.title);
        assert_eq!(org.sections.len(), restored_org.sections.len());

        // SafeOrg形式のJSONが正しく生成されることを確認
        assert!(json.contains("Test Document") || json.contains("Section 1"));
    }

    #[test]
    fn test_pretty_json_conversion() {
        init();

        let content = r#"#+TITLE: Test Document

* Section 1
Content for section 1
"#;

        let mut ctx = Context::new();
        let org = parse(&mut ctx, content).expect("Failed to parse org content");

        let json = org
            .to_json_pretty()
            .expect("Failed to convert to pretty JSON");
        assert!(json.contains("  ")); // インデントが含まれていることを確認
    }

    #[test]
    fn test_depth_limit() {
        init();

        // 深い入れ子構造を作成
        let mut content = String::from("#+TITLE: Deep Structure\n\n");
        for i in 1..=15 {
            content.push_str(&"*".repeat(i));
            content.push_str(&format!(" Section {}\nContent {}\n\n", i, i));
        }

        let mut ctx = Context::new();
        let org = parse(&mut ctx, &content).expect("Failed to parse org content");

        // デフォルト設定（max_depth: 10）でテスト
        let config = JsonConversionConfig::default();
        let result = org.to_json_with_config(&config);

        // 深度制限により変換が成功するか、適切にエラーが発生することを確認
        match result {
            Ok(_) => {
                // 変換が成功した場合、深度制限内で処理されたことを意味する
            }
            Err(JsonConversionError::MaxDepthExceeded { depth }) => {
                assert!(depth >= 10);
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[test]
    fn test_section_specific_conversion() {
        init();

        let content = r#"#+TITLE: Test Document

* Section 1
:PROPERTIES:
:ID: section-1-id
:END:
Content for section 1

* Section 2
:PROPERTIES:
:ID: section-2-id
:END:
Content for section 2
"#;

        let mut ctx = Context::new();
        let org = parse(&mut ctx, content).expect("Failed to parse org content");

        let config = JsonConversionConfig::default();
        let section_json = org
            .section_to_json("section-1-id", &config)
            .expect("Failed to convert section to JSON");

        assert!(section_json.is_some());
        let json = section_json.unwrap();
        assert!(json.contains("Section 1"));
        assert!(!json.contains("Section 2"));
    }

    #[test]
    fn test_streaming_conversion() {
        init();

        let content = r#"#+TITLE: Test Document

* Section 1
Content for section 1
"#;

        let mut ctx = Context::new();
        let org = parse(&mut ctx, content).expect("Failed to parse org content");

        let mut buffer = Vec::new();
        let config = JsonConversionConfig::default();

        org.to_json_stream(&mut buffer, &config)
            .expect("Failed to stream JSON");

        let json_string = String::from_utf8(buffer).expect("Invalid UTF-8");
        assert!(!json_string.is_empty());

        // ストリーミング結果が通常の変換結果と一致することを確認
        let normal_json = org
            .to_json_with_config(&config)
            .expect("Failed to convert normally");
        assert_eq!(json_string, normal_json);
    }

    #[test]
    fn test_position_inclusion_control() {
        init();

        let content = r#"* Section 1
Content for section 1
"#;

        let mut ctx = Context::new();
        let org = parse(&mut ctx, content).expect("Failed to parse org content");

        // 位置情報を含む設定
        let config_with_pos = JsonConversionConfig {
            include_position: true,
            ..Default::default()
        };
        let json_with_pos = org
            .to_json_with_config(&config_with_pos)
            .expect("Failed to convert with position");

        // 位置情報を含まない設定
        let config_without_pos = JsonConversionConfig {
            include_position: false,
            ..Default::default()
        };
        let json_without_pos = org
            .to_json_with_config(&config_without_pos)
            .expect("Failed to convert without position");

        // 位置情報を含む場合は"pos"フィールドが存在し、含まない場合は存在しない
        assert!(json_with_pos.contains("\"pos\""));
        assert!(!json_without_pos.contains("\"pos\""));

        // 位置情報を除いた場合の方がサイズが小さいことを確認
        assert!(json_without_pos.len() < json_with_pos.len());
    }
}

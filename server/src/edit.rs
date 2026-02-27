use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum TargetEntry {
    Id {
        id: String,
    },
    Path {
        path: Vec<String>,
    },
    Line {
        line_number: usize,
        expected_title: Option<String>,
    },
}

impl TargetEntry {
    pub fn resolve(&self, content: &str) -> Result<usize> {
        let mut org_ctx = org_parser::Context::new();
        match self {
            TargetEntry::Id { id } => {
                let org = org_parser::parse(&mut org_ctx, content)
                    .map_err(|e| anyhow::anyhow!("Parse error: {:?}", e))?;
                if let Some(line) = find_section_by_id(&org.sections, id) {
                    Ok(line)
                } else {
                    Err(anyhow::anyhow!("Entry with id {} not found", id))
                }
            }
            TargetEntry::Path { path } => {
                let org = org_parser::parse(&mut org_ctx, content)
                    .map_err(|e| anyhow::anyhow!("Parse error: {:?}", e))?;
                if let Some(line) = find_section_by_path(&org.sections, path, 0) {
                    Ok(line)
                } else {
                    Err(anyhow::anyhow!("Entry with path {:?} not found", path))
                }
            }
            TargetEntry::Line {
                line_number,
                expected_title,
            } => {
                if let Some(expected) = expected_title {
                    let lines: Vec<&str> = content.split('\n').collect();
                    let target_idx = line_number.saturating_sub(1);
                    if target_idx >= lines.len() {
                        return Err(anyhow::anyhow!("Line {} out of bounds", line_number));
                    }
                    let line = lines[target_idx];

                    let org = org_parser::parse(&mut org_ctx, &format!("{}\n", line))
                        .map_err(|e| anyhow::anyhow!("Parse error: {:?}", e))?;
                    if let Some(sec) = org.sections.first() {
                        if sec.title.trim() != expected.trim() {
                            return Err(anyhow::anyhow!(
                                "Headline title mismatch. Expected '{}', got '{}'",
                                expected,
                                sec.title.trim()
                            ));
                        }
                    } else {
                        return Err(anyhow::anyhow!("Line {} is not a headline", line_number));
                    }
                }
                Ok(*line_number)
            }
        }
    }
}

fn get_level(sec: &org_parser::Section) -> usize {
    sec.headline_symbol.len()
}

fn find_section_by_id(sections: &[org_parser::Section], id: &str) -> Option<usize> {
    for sec in sections {
        if sec.id == id {
            return Some(sec.pos.line);
        }
        if let Some(line) = find_section_by_id(&sec.sections, id) {
            return Some(line);
        }
    }
    None
}

fn find_section_by_path(
    sections: &[org_parser::Section],
    path: &[String],
    depth: usize,
) -> Option<usize> {
    if path.is_empty() || depth >= path.len() {
        return None;
    }

    // org_parser parses sections mostly as a flat list, but occasionally might nest them.
    // To support a hierarchy like "Parent" -> "Child", we find "Parent" and then look for "Child"
    // among subsequent sections that have a STRICTLY GREATER level, until we hit a section
    // with level <= Parent's level.

    let target_title = &path[depth];

    for (i, sec) in sections.iter().enumerate() {
        if sec.title.trim() == target_title.trim() {
            if depth == path.len() - 1 {
                return Some(sec.pos.line);
            }

            let parent_level = get_level(sec);

            // Sub-headlines could be strictly inside `sec.sections` or coming after `sec` in `sections`.
            // First check explicitly nested ones:
            if let Some(line) = find_section_by_path(&sec.sections, path, depth + 1) {
                return Some(line);
            }

            // Then check siblings in the flat list that act as children (level > parent_level)
            let mut children_flat = Vec::new();
            for sibling in &sections[i + 1..] {
                if get_level(sibling) <= parent_level {
                    break; // Reached next sibling or upper level, stop collecting children
                }
                children_flat.push(sibling.clone());
            }

            if let Some(line) = find_section_by_path(&children_flat, path, depth + 1) {
                return Some(line);
            }
        } else {
            // Search inside explicitly nested sections (if any)
            if let Some(line) = find_section_by_path(&sec.sections, path, depth) {
                return Some(line);
            }
        }
    }
    None
}

fn find_status_at_line(sections: &[org_parser::Section], line: usize) -> Option<String> {
    for section in sections {
        if section.pos.line == line {
            return section.todo_status.clone();
        }
        if let Some(res) = find_status_at_line(&section.sections, line) {
            return Some(res);
        }
    }
    None
}

pub async fn do_update_todo_status(
    resolved: &Path,
    target: &TargetEntry,
    new_status: &str,
) -> Result<String> {
    let content = fs::read_to_string(resolved).await?;
    let headline_line_number = target.resolve(&content)?;
    let target_idx = headline_line_number.saturating_sub(1);

    let mut lines: Vec<&str> = content.split('\n').collect();
    if target_idx >= lines.len() {
        return Err(anyhow::anyhow!(
            "Line {} out of bounds",
            headline_line_number
        ));
    }

    let target_line = lines[target_idx];
    let parts: Vec<&str> = target_line.splitn(2, ' ').collect();
    if parts.is_empty() || !parts[0].starts_with('*') {
        return Err(anyhow::anyhow!(
            "Line {} is not a headline",
            headline_line_number
        ));
    }

    let mut existing_status = None;
    if let Ok(org) = org_parser::parse(&mut org_parser::Context::new(), &content) {
        existing_status = find_status_at_line(&org.sections, headline_line_number);
    }

    let prefix = format!("{} ", parts[0]);
    let mut new_line = target_line.to_string();

    let new_status_clean = new_status.trim();

    if let Some(old_status) = existing_status {
        if target_line[prefix.len()..].starts_with(&old_status) {
            let rest_idx = prefix.len() + old_status.len();
            let rest = target_line[rest_idx..].trim_start();

            if new_status_clean.is_empty() {
                new_line = format!("{}{}", prefix, rest);
            } else {
                new_line = format!(
                    "{}{}{}",
                    prefix,
                    new_status_clean,
                    if rest.is_empty() { "" } else { " " }
                );
                new_line.push_str(rest);
            }
        }
    } else {
        // No existing status, insert after asterisks
        let rest = parts.get(1).unwrap_or(&"").trim_start();
        if new_status_clean.is_empty() {
            new_line = format!("{}{}", prefix, rest);
        } else {
            new_line = format!(
                "{}{}{}",
                prefix,
                new_status_clean,
                if rest.is_empty() { "" } else { " " }
            );
            new_line.push_str(rest);
        }
    }

    lines[target_idx] = &new_line;

    fs::write(resolved, lines.join("\n")).await?;

    Ok(format!(
        "Successfully updated line {} to status '{}'",
        headline_line_number, new_status_clean
    ))
}

pub async fn do_append_task(
    resolved: &Path,
    title: &str,
    status: Option<&str>,
    tags: Option<&[String]>,
) -> Result<String> {
    let mut new_line = "* ".to_string();
    if let Some(st) = status
        && !st.trim().is_empty()
    {
        new_line.push_str(&format!("{} ", st.trim()));
    }
    new_line.push_str(title.trim());

    if let Some(tgs) = tags
        && !tgs.is_empty()
    {
        new_line.push_str(&format!(" :{}:", tgs.join(":")));
    }

    let mut content = fs::read_to_string(resolved)
        .await
        .unwrap_or_else(|_| String::new());

    if !content.ends_with('\n') && !content.is_empty() {
        content.push('\n');
    }
    content.push_str(&new_line);
    content.push('\n');

    fs::write(resolved, content).await?;

    Ok(format!("Successfully appended task '{}'", title))
}

pub async fn do_update_scheduling(
    resolved: &Path,
    target: &TargetEntry,
    scheduling_type: &str,
    timestamp: &str,
) -> Result<String> {
    let content = fs::read_to_string(resolved).await?;
    let headline_line_number = target.resolve(&content)?;
    let target_idx = headline_line_number.saturating_sub(1);
    let s_type = scheduling_type.trim().to_uppercase();

    if s_type != "SCHEDULED" && s_type != "DEADLINE" {
        return Err(anyhow::anyhow!(
            "scheduling_type must be SCHEDULED or DEADLINE"
        ));
    }

    let mut lines: Vec<&str> = content.split('\n').collect();
    if target_idx >= lines.len() {
        return Err(anyhow::anyhow!(
            "Line {} out of bounds",
            headline_line_number
        ));
    }

    let target_line = lines[target_idx];
    if !target_line.starts_with('*') {
        return Err(anyhow::anyhow!(
            "Line {} is not a headline",
            headline_line_number
        ));
    }

    let new_schedule_str = format!("{}: {}", s_type, timestamp);

    // Find existing line that exactly matches our prefix.
    let mut found_existing_line = None;
    for (i, line) in lines.iter().enumerate().skip(target_idx + 1) {
        let line_trim = line.trim_start();
        if line_trim.starts_with('*') {
            break; // next headline
        }
        if line_trim.starts_with(&s_type) {
            found_existing_line = Some(i);
            break;
        }
    }

    let new_schedule_str_owned = new_schedule_str;

    if let Some(idx) = found_existing_line {
        // Replace existing schedule
        lines[idx] = &new_schedule_str_owned;
    } else {
        // Insert right after headline
        lines.insert(target_idx + 1, &new_schedule_str_owned);
    }

    fs::write(resolved, lines.join("\n")).await?;

    Ok(format!("Successfully updated {}", s_type))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    async fn run_update_todo_test(content: &str, line_num: usize, new_status: &str) -> String {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", content).unwrap();

        do_update_todo_status(
            file.path(),
            &TargetEntry::Line {
                line_number: line_num,
                expected_title: None,
            },
            new_status,
        )
        .await
        .unwrap();

        fs::read_to_string(file.path()).await.unwrap()
    }

    #[tokio::test]
    async fn test_update_todo_status_priority() {
        let content = "* TODO [#A] Fix this bug\n";
        let out = run_update_todo_test(content, 1, "DONE").await;
        // The expected result should be replacing TODO with DONE
        // even though there is a priority tag.
        assert_eq!(out, "* DONE [#A] Fix this bug\n");
    }

    #[tokio::test]
    async fn test_update_todo_status_simple() {
        let content = "* TODO Fix this bug\n";
        let out = run_update_todo_test(content, 1, "DONE").await;
        assert_eq!(out, "* DONE Fix this bug\n");
    }
}

pub fn get_headline_level(line: &str) -> Option<usize> {
    if line.starts_with('*') {
        let level = line.chars().take_while(|&c| c == '*').count();
        if line[level..].starts_with(' ') || line[level..].is_empty() {
            return Some(level);
        }
    }
    None
}

pub fn find_subtree_end(lines: &[&str], start_idx: usize) -> usize {
    let Some(level) = get_headline_level(lines[start_idx]) else {
        return start_idx + 1;
    };

    for (i, line) in lines.iter().enumerate().skip(start_idx + 1) {
        if let Some(l) = get_headline_level(line)
            && l <= level {
                return i;
            }
    }
    lines.len()
}

pub fn find_next_headline(lines: &[&str], start_idx: usize) -> usize {
    for (i, line) in lines.iter().enumerate().skip(start_idx + 1) {
        if get_headline_level(line).is_some() {
            return i;
        }
    }
    lines.len()
}

pub async fn do_insert_content(
    resolved: &Path,
    target: &TargetEntry,
    content_to_insert: &str,
) -> Result<String> {
    let content = fs::read_to_string(resolved).await?;
    let target_line_number = target.resolve(&content)?;
    let target_idx = target_line_number.saturating_sub(1);

    let mut lines: Vec<&str> = content.split('\n').collect();
    if target_idx >= lines.len() {
        return Err(anyhow::anyhow!("Line {} out of bounds", target_line_number));
    }

    let insert_str = content_to_insert.to_string();

    // Insert after the target_idx
    lines.insert(target_idx + 1, &insert_str);

    fs::write(resolved, lines.join("\n")).await?;

    Ok(format!(
        "Successfully inserted content after line {}",
        target_line_number
    ))
}

pub async fn do_update_headline(
    resolved: &Path,
    target: &TargetEntry,
    new_title: &str,
    new_body: &str,
) -> Result<String> {
    let content = fs::read_to_string(resolved).await?;
    let headline_line_number = target.resolve(&content)?;
    let target_idx = headline_line_number.saturating_sub(1);

    let lines: Vec<&str> = content.split('\n').collect();
    if target_idx >= lines.len() {
        return Err(anyhow::anyhow!(
            "Line {} out of bounds",
            headline_line_number
        ));
    }

    if get_headline_level(lines[target_idx]).is_none() {
        return Err(anyhow::anyhow!(
            "Line {} is not a headline",
            headline_line_number
        ));
    }

    // Parse the existing headline to keep metadata
    let mut org_ctx = org_parser::Context::new();
    let old_headline_text = lines[target_idx];
    let parse_text = format!("{}\n", old_headline_text);

    let mut new_headline = old_headline_text.to_string(); // fallback
    if let Ok(org) = org_parser::parse(&mut org_ctx, &parse_text) {
        if let Some(section) = org.sections.first() {
            let mut rebuilt = section.headline_symbol.clone();
            rebuilt.push(' ');
            if let Some(todo) = &section.todo_status {
                rebuilt.push_str(todo);
                rebuilt.push(' ');
            }
            if let Some(prio) = &section.priority {
                rebuilt.push_str(&format!("[#{}] ", prio));
            }
            rebuilt.push_str(new_title.trim());
            if !section.tags.is_empty() {
                rebuilt.push_str(&format!(" :{}:", section.tags.join(":")));
            }
            new_headline = rebuilt;
        }
    } else {
        // Fallback if parsing fails for some reason
        let level = get_headline_level(old_headline_text).unwrap();
        new_headline = format!("{} {}", "*".repeat(level), new_title.trim());
    }

    let end_idx = find_next_headline(&lines, target_idx);

    // Replace lines from target_idx to end_idx-1
    let mut new_lines = Vec::new();
    new_lines.extend_from_slice(&lines[..target_idx]);

    let new_headline_owned = new_headline;
    let new_body_owned = new_body.to_string();

    new_lines.push(&new_headline_owned);
    if !new_body_owned.is_empty() {
        new_lines.push(&new_body_owned);
    }

    new_lines.extend_from_slice(&lines[end_idx..]);

    fs::write(resolved, new_lines.join("\n")).await?;

    Ok(format!(
        "Successfully updated headline at line {}",
        headline_line_number
    ))
}

pub async fn do_delete_headline(resolved: &Path, target: &TargetEntry) -> Result<String> {
    let content = fs::read_to_string(resolved).await?;
    let headline_line_number = target.resolve(&content)?;
    let target_idx = headline_line_number.saturating_sub(1);

    let lines: Vec<&str> = content.split('\n').collect();
    if target_idx >= lines.len() {
        return Err(anyhow::anyhow!(
            "Line {} out of bounds",
            headline_line_number
        ));
    }

    if get_headline_level(lines[target_idx]).is_none() {
        return Err(anyhow::anyhow!(
            "Line {} is not a headline",
            headline_line_number
        ));
    }

    let end_idx = find_subtree_end(&lines, target_idx);

    let mut new_lines = Vec::new();
    new_lines.extend_from_slice(&lines[..target_idx]);
    new_lines.extend_from_slice(&lines[end_idx..]);

    fs::write(resolved, new_lines.join("\n")).await?;

    Ok(format!(
        "Successfully deleted headline and its subtree at line {}",
        headline_line_number
    ))
}

#[cfg(test)]
mod additional_tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_do_insert_content() {
        let content = "* Headline 1\n* Headline 2\n";
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", content).unwrap();

        do_insert_content(
            file.path(),
            &TargetEntry::Line {
                line_number: 1,
                expected_title: None,
            },
            "body text",
        )
        .await
        .unwrap();

        let out = fs::read_to_string(file.path()).await.unwrap();
        assert_eq!(out, "* Headline 1\nbody text\n* Headline 2\n");
    }

    #[tokio::test]
    async fn test_do_update_headline() {
        let content = "* TODO [#A] Old Title :tag:\nold body\n** Subhead\n";
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", content).unwrap();

        do_update_headline(
            file.path(),
            &TargetEntry::Line {
                line_number: 1,
                expected_title: None,
            },
            "New Title",
            "new body text",
        )
        .await
        .unwrap();

        let out = fs::read_to_string(file.path()).await.unwrap();
        assert_eq!(
            out,
            "* TODO [#A] New Title :tag:\nnew body text\n** Subhead\n"
        );
    }

    #[tokio::test]
    async fn test_do_delete_headline() {
        let content = "* Head 1\n** Sub 1\n*** SubSub 1\n* Head 2\n";
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", content).unwrap();

        do_delete_headline(
            file.path(),
            &TargetEntry::Line {
                line_number: 1,
                expected_title: None,
            },
        )
        .await
        .unwrap();

        let out = fs::read_to_string(file.path()).await.unwrap();
        assert_eq!(out, "* Head 2\n");
    }

    #[test]
    fn test_target_entry_resolution() {
        let content = "
* Project Alpha
:PROPERTIES:
:ID:       12345
:END:
** Task 1
body
* Project Beta
** Task 1
";
        let mut org_ctx = org_parser::Context::new();
        let _org = org_parser::parse(&mut org_ctx, content).unwrap();

        let target_id = TargetEntry::Id {
            id: "12345".to_string(),
        };
        assert_eq!(target_id.resolve(content).unwrap(), 2);

        let target_path = TargetEntry::Path {
            path: vec!["Project Beta".to_string(), "Task 1".to_string()],
        };
        assert_eq!(target_path.resolve(content).unwrap(), 9);

        let target_line_ok = TargetEntry::Line {
            line_number: 8,
            expected_title: Some("Project Beta".to_string()),
        };
        assert_eq!(target_line_ok.resolve(content).unwrap(), 8);

        let target_line_err = TargetEntry::Line {
            line_number: 8,
            expected_title: Some("Wrong Title".to_string()),
        };
        assert!(target_line_err.resolve(content).is_err());
    }

    #[test]
    fn test_target_entry_edge_cases() {
        let content = "* Root\n** Child 1\n*** Grandchild\n** Child 2\n* Root 2\n";

        // Path resolution for deep nesting where sections are siblings under Root
        let target_deep_path = TargetEntry::Path {
            path: vec![
                "Root".to_string(),
                "Child 1".to_string(),
                "Grandchild".to_string(),
            ],
        };
        assert_eq!(target_deep_path.resolve(content).unwrap(), 3);

        let target_missing_path = TargetEntry::Path {
            path: vec![
                "Root".to_string(),
                "Child 2".to_string(),
                "Grandchild".to_string(),
            ],
        };
        assert!(target_missing_path.resolve(content).is_err());

        // Out of bounds checking
        let target_oob_line = TargetEntry::Line {
            line_number: 100,
            expected_title: Some("Non-existent".to_string()),
        };
        let err = target_oob_line.resolve(content).unwrap_err();
        assert_eq!(err.to_string(), "Line 100 out of bounds");

        // Not a headline checking
        let not_headline_line = TargetEntry::Line {
            line_number: 1,
            expected_title: Some("Not a headline, this is text".to_string()),
        };
        // At line 1 it is "* Root", which doesn't match the title
        let err2 = not_headline_line.resolve(content).unwrap_err();
        assert!(err2.to_string().contains("Headline title mismatch"));
    }

    #[test]
    fn test_helpers() {
        let lines = vec!["* H1", "body", "** H2", "*** H3", "* H4"];
        assert_eq!(get_headline_level(lines[0]), Some(1));
        assert_eq!(get_headline_level(lines[1]), None);
        assert_eq!(find_subtree_end(&lines, 0), 4);
        assert_eq!(find_subtree_end(&lines, 2), 4);
        assert_eq!(find_next_headline(&lines, 0), 2);
    }
}

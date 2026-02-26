use anyhow::Result;
use std::path::Path;
use tokio::fs;

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
    headline_line_number: usize,
    new_status: &str,
) -> Result<String> {
    let target_idx = headline_line_number.saturating_sub(1);
    let content = fs::read_to_string(resolved).await?;

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
    headline_line_number: usize,
    scheduling_type: &str,
    timestamp: &str,
) -> Result<String> {
    let target_idx = headline_line_number.saturating_sub(1);
    let s_type = scheduling_type.trim().to_uppercase();

    if s_type != "SCHEDULED" && s_type != "DEADLINE" {
        return Err(anyhow::anyhow!(
            "scheduling_type must be SCHEDULED or DEADLINE"
        ));
    }

    let content = fs::read_to_string(resolved).await?;

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

        do_update_todo_status(file.path(), line_num, new_status)
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

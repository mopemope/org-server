use org_parser::{Context, parse};
use std::time::Instant;

fn simple_org_content() -> &'static str {
    r#"#+TITLE: Simple Test

* TODO First task
This is a simple task.

* DONE Second task  
This task is completed.
"#
}

fn medium_org_content() -> String {
    let mut content = String::new();
    content.push_str("#+TITLE: Medium Test\n\n");

    for i in 0..50 {
        content.push_str(&format!("* TODO Task {} :tag{}:\n", i + 1, i % 3));
        content.push_str(&format!(
            "SCHEDULED: <2024-12-{:02} Mon 09:00>\n",
            (i % 28) + 1
        ));
        content.push_str(":PROPERTIES:\n");
        content.push_str(&format!(":ID: task-{:03}\n", i));
        content.push_str(":END:\n");
        content.push_str(&format!(
            "This is task number {} with some content.\n\n",
            i + 1
        ));
    }

    content
}

fn large_org_content() -> String {
    let mut content = String::new();
    content.push_str("#+TITLE: Large Test\n\n");

    for i in 0..200 {
        content.push_str(&format!("* TODO Task {} :tag{}:work:\n", i + 1, i % 5));
        content.push_str(&format!(
            "SCHEDULED: <2024-12-{:02} Mon 09:00>\n",
            (i % 28) + 1
        ));
        content.push_str(":PROPERTIES:\n");
        content.push_str(&format!(":ID: task-{:04}\n", i));
        for j in 0..3 {
            content.push_str(&format!(":PROP_{}: value_{}_{}\n", j, i, j));
        }
        content.push_str(":END:\n");
        content.push_str(&format!(
            "This is task number {} with detailed content.\n",
            i + 1
        ));
        content.push_str("More text here to make it realistic.\n\n");

        if i % 10 == 0 {
            content.push_str(&format!("** Subtask {}.1\n", i + 1));
            content.push_str("Subtask content.\n\n");
        }
    }

    content
}

fn main() {
    println!("=== Org Parser Performance Analysis ===\n");

    // Simple test
    println!("1. Simple content test:");
    let simple_content = simple_org_content();
    println!("Content size: {} bytes", simple_content.len());

    let start = Instant::now();
    let mut ctx = Context::new();
    match parse(&mut ctx, simple_content) {
        Ok(org) => {
            let duration = start.elapsed();
            println!("Parse time: {:?}", duration);
            println!("Sections: {}", org.sections.len());
            println!("Keywords: {}", org.keywords.len());
            println!("Properties: {}", org.properties.len());
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }

    println!("\n2. Medium content test (50 sections):");
    let medium_content = medium_org_content();
    println!("Content size: {} bytes", medium_content.len());

    let start = Instant::now();
    let mut ctx = Context::new();
    match parse(&mut ctx, &medium_content) {
        Ok(org) => {
            let duration = start.elapsed();
            println!("Parse time: {:?}", duration);
            println!("Sections: {}", org.sections.len());
            println!("Total reminders: {}", org.get_reminders().len());
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }

    println!("\n3. Large content test (200 sections):");
    let large_content = large_org_content();
    println!("Content size: {} bytes", large_content.len());

    let start = Instant::now();
    let mut ctx = Context::new();
    match parse(&mut ctx, &large_content) {
        Ok(org) => {
            let duration = start.elapsed();
            println!("Parse time: {:?}", duration);
            println!("Sections: {}", org.sections.len());
            println!("Total reminders: {}", org.get_reminders().len());
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
            return;
        }
    }

    // 複数回実行して平均を取る
    println!("\n4. Multiple runs test (medium content):");
    let runs = 10;
    let mut total_duration = std::time::Duration::new(0, 0);

    for i in 0..runs {
        let start = Instant::now();
        let mut ctx = Context::new();
        match parse(&mut ctx, &medium_content) {
            Ok(_) => {
                total_duration += start.elapsed();
            }
            Err(e) => {
                println!("Parse error on run {}: {:?}", i, e);
                return;
            }
        }
    }

    let avg_duration = total_duration / runs;
    println!("Average parse time over {} runs: {:?}", runs, avg_duration);
    println!("Total time: {:?}", total_duration);
}

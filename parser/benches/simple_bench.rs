use criterion::{Criterion, black_box, criterion_group, criterion_main};
use org_parser::{Context, parse};
use std::time::Instant;

// 非常にシンプルなorgコンテンツ
fn simple_org_content() -> &'static str {
    r#"#+TITLE: Simple Test

* TODO First task
This is a simple task.

* DONE Second task  
This task is completed.
"#
}

// 中程度のorgコンテンツ
fn medium_org_content() -> String {
    let mut content = String::new();
    content.push_str("#+TITLE: Medium Test\n\n");

    for i in 0..10 {
        content.push_str(&format!("* TODO Task {}\n", i + 1));
        content.push_str(&format!("This is task number {}.\n\n", i + 1));
    }

    content
}

fn bench_simple_parse(c: &mut Criterion) {
    let content = simple_org_content();

    c.bench_function("parse_simple", |b| {
        b.iter(|| {
            let mut ctx = Context::new();
            parse(black_box(&mut ctx), black_box(content)).unwrap()
        })
    });
}

fn bench_medium_parse(c: &mut Criterion) {
    let content = medium_org_content();

    c.bench_function("parse_medium_10_tasks", |b| {
        b.iter(|| {
            let mut ctx = Context::new();
            parse(black_box(&mut ctx), black_box(&content)).unwrap()
        })
    });
}

// 手動でのタイミング測定
#[allow(dead_code)]
fn manual_timing_test() {
    println!("=== Manual Timing Test ===");

    let simple_content = simple_org_content();
    let start = Instant::now();
    let mut ctx = Context::new();
    let result = parse(&mut ctx, simple_content);
    let duration = start.elapsed();

    match result {
        Ok(org) => {
            println!("Simple parse: {:?}", duration);
            println!("Sections: {}", org.sections.len());
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }

    let medium_content = medium_org_content();
    let start = Instant::now();
    let mut ctx = Context::new();
    let result = parse(&mut ctx, &medium_content);
    let duration = start.elapsed();

    match result {
        Ok(org) => {
            println!("Medium parse: {:?}", duration);
            println!("Sections: {}", org.sections.len());
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }
}

criterion_group!(benches, bench_simple_parse, bench_medium_parse);
criterion_main!(benches);

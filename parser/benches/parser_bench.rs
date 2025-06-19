use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use org_parser::{Context, parse};
use std::fs;
use std::path::Path;

// 様々なサイズのorgファイルを生成する関数
fn generate_org_content(sections: usize, properties_per_section: usize) -> String {
    let mut content = String::new();

    // ファイルレベルのプロパティ
    content.push_str(":PROPERTIES:\n");
    content.push_str(":ID: file-id-12345\n");
    content.push_str(":TITLE: Test Org File\n");
    content.push_str(":END:\n\n");

    // キーワード
    content.push_str("#+TITLE: Performance Test File\n");
    content.push_str("#+TODO: TODO DOING | DONE\n\n");

    for i in 0..sections {
        // ヘッドライン
        content.push_str(&format!("* TODO Section {} :tag1:tag2:\n", i + 1));
        content.push_str(&format!(
            "SCHEDULED: <2024-12-{:02} Mon 09:00>\n",
            (i % 28) + 1
        ));

        // プロパティ
        if properties_per_section > 0 {
            content.push_str(":PROPERTIES:\n");
            for j in 0..properties_per_section {
                content.push_str(&format!(":PROP_{}: value_{}\n", j, j));
            }
            content.push_str(":END:\n");
        }

        // ドロワー
        content.push_str(":LOGBOOK:\n");
        content.push_str("- State \"DONE\" from \"TODO\" [2024-01-01 Mon 10:00]\n");
        content.push_str(":END:\n");

        // コンテンツ
        content.push_str("This is some content for the section.\n");
        content.push_str("It contains multiple lines and [[https://example.com][links]].\n\n");

        // サブセクション
        if i % 5 == 0 {
            content.push_str(&format!("** Subsection {}.1\n", i + 1));
            content.push_str("Some subsection content.\n\n");
        }
    }

    content
}

fn bench_parse_small(c: &mut Criterion) {
    let content = generate_org_content(10, 3);

    c.bench_function("parse_small_10_sections", |b| {
        b.iter(|| {
            let mut ctx = Context::new();
            parse(black_box(&mut ctx), black_box(&content)).unwrap()
        })
    });
}

fn bench_parse_medium(c: &mut Criterion) {
    let content = generate_org_content(100, 5);

    c.bench_function("parse_medium_100_sections", |b| {
        b.iter(|| {
            let mut ctx = Context::new();
            parse(black_box(&mut ctx), black_box(&content)).unwrap()
        })
    });
}

fn bench_parse_large(c: &mut Criterion) {
    let content = generate_org_content(1000, 10);

    c.bench_function("parse_large_1000_sections", |b| {
        b.iter(|| {
            let mut ctx = Context::new();
            parse(black_box(&mut ctx), black_box(&content)).unwrap()
        })
    });
}

fn bench_parse_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_scaling");

    for sections in [10, 50, 100, 500, 1000].iter() {
        let content = generate_org_content(*sections, 5);

        group.bench_with_input(
            BenchmarkId::new("sections", sections),
            sections,
            |b, _sections| {
                b.iter(|| {
                    let mut ctx = Context::new();
                    parse(black_box(&mut ctx), black_box(&content)).unwrap()
                })
            },
        );
    }
    group.finish();
}

fn bench_parse_properties_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_properties_scaling");

    for props in [1, 5, 10, 20, 50].iter() {
        let content = generate_org_content(100, *props);

        group.bench_with_input(BenchmarkId::new("properties", props), props, |b, _props| {
            b.iter(|| {
                let mut ctx = Context::new();
                parse(black_box(&mut ctx), black_box(&content)).unwrap()
            })
        });
    }
    group.finish();
}

// 実際のorgファイルがある場合のベンチマーク
fn bench_real_files(c: &mut Criterion) {
    let test_files = [
        "test_data/small.org",
        "test_data/medium.org",
        "test_data/large.org",
    ];

    for file_path in test_files.iter() {
        if Path::new(file_path).exists() {
            let content = fs::read_to_string(file_path).unwrap();
            let file_name = Path::new(file_path).file_stem().unwrap().to_str().unwrap();

            c.bench_function(&format!("parse_real_{}", file_name), |b| {
                b.iter(|| {
                    let mut ctx = Context::new();
                    parse(black_box(&mut ctx), black_box(&content)).unwrap()
                })
            });
        }
    }
}

criterion_group!(
    benches,
    bench_parse_small,
    bench_parse_medium,
    bench_parse_large,
    bench_parse_scaling,
    bench_parse_properties_scaling,
    bench_real_files
);
criterion_main!(benches);

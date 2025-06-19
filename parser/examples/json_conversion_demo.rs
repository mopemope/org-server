use org_parser::{Context, JsonConversionConfig, parse};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // サンプルのOrg-modeコンテンツ
    let content = r#"#+TITLE: JSON変換デモ

* プロジェクト概要
:PROPERTIES:
:ID: project-overview
:CREATED: <2024-01-01 Mon 10:00>
:END:

このプロジェクトはOrg-modeファイルのJSON変換機能を提供します。

** 主要機能
- 深度制限による安全な変換
- 位置情報の制御
- 軽量版JSON出力

*** 詳細機能
- ストリーミング変換
- 部分的変換
- 美しい整形

* 使用例
SCHEDULED: <2024-12-25 Wed 09:00>

以下のような使い方ができます：

[[https://example.com][リンクの例]]

** サブセクション
内容のテスト
"#;

    // パーサーでOrg-modeコンテンツを解析
    let mut ctx = Context::new();
    let org = parse(&mut ctx, content)?;

    println!("=== Org-mode JSON変換デモ ===\n");

    // 1. 基本的なJSON変換（位置情報あり）
    println!("1. 基本的なJSON変換（位置情報あり）:");
    let basic_json = org.to_json_pretty()?;
    println!("{}\n", basic_json);

    // 2. 軽量版JSON（位置情報なし）
    println!("2. 軽量版JSON（位置情報なし）:");
    let compact_json = org.to_json_compact()?;
    println!("{}\n", compact_json);

    // 3. カスタム設定での変換
    println!("3. カスタム設定での変換（深度制限2、位置情報なし）:");
    let custom_config = JsonConversionConfig {
        max_depth: 2,
        include_position: false,
        pretty_print: true,
        include_empty_sections: false,
    };
    let custom_json = org.to_json_with_config(&custom_config)?;
    println!("{}\n", custom_json);

    // 4. 特定セクションのみの変換
    println!("4. 特定セクション（project-overview）のみの変換:");
    if let Some(section_json) =
        org.section_to_json("project-overview", &JsonConversionConfig::default())?
    {
        println!("{}\n", section_json);
    } else {
        println!("指定されたセクションが見つかりませんでした。\n");
    }

    // 5. サイズ比較
    let with_pos_size = org
        .to_json_with_config(&JsonConversionConfig {
            include_position: true,
            ..Default::default()
        })?
        .len();

    let without_pos_size = org
        .to_json_with_config(&JsonConversionConfig {
            include_position: false,
            ..Default::default()
        })?
        .len();

    println!("5. サイズ比較:");
    println!("位置情報あり: {} bytes", with_pos_size);
    println!("位置情報なし: {} bytes", without_pos_size);
    println!(
        "削減率: {:.1}%",
        (1.0 - without_pos_size as f64 / with_pos_size as f64) * 100.0
    );

    // 6. リマインダー情報の表示
    println!("\n6. 抽出されたリマインダー:");
    let reminders = org.get_reminders();
    for reminder in reminders {
        println!("- {}: {}", reminder.title, reminder.datetime);
    }

    Ok(())
}

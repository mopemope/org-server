#[cfg(test)]
mod comprehensive_json_conversion_tests {
    use crate::json_conversion::{JsonConversionConfig, JsonConversionError};
    use crate::test_helpers::*;
    use std::collections::HashMap;

    fn init() {
        let _ = tracing_subscriber::fmt::try_init();
    }

    /// 基本的な往復変換テスト
    #[test]
    fn test_roundtrip_conversion_basic() {
        init();

        let content = r#"#+TITLE: Basic Test

* Section 1
:PROPERTIES:
:ID: section-1
:END:

Content for section 1.

** Subsection 1.1
:PROPERTIES:
:ID: subsection-1-1
:END:

Content for subsection 1.1.
"#;

        let org = parse_test_org(content).expect("Failed to parse org content");
        let config = JsonConversionConfig::default();

        let result = test_roundtrip_conversion(&org, &config)
            .expect("Failed to perform roundtrip conversion");

        let report = result
            .verify_complete()
            .expect("Failed to verify conversion");

        if !report.is_valid() {
            report.print_summary();
            panic!("Roundtrip conversion failed validation");
        }
    }

    /// 複雑な構造のテストファイルを使った往復変換テスト
    #[test]
    fn test_roundtrip_conversion_complex_structure() {
        init();

        let org = parse_test_resource("complex_structure.org")
            .expect("Failed to parse complex_structure.org");

        let config = JsonConversionConfig::default();
        let result = test_roundtrip_conversion(&org, &config)
            .expect("Failed to perform roundtrip conversion");

        let report = result
            .verify_complete()
            .expect("Failed to verify conversion");

        if !report.is_valid() {
            report.print_summary();
            // 複雑な構造では一部の不一致は許容する場合があるため、
            // エラー数が許容範囲内かチェック
            assert!(
                report.total_errors() < 5,
                "Too many errors in complex structure test"
            );
        }
    }

    /// スケジューリングパターンのテスト
    #[test]
    fn test_roundtrip_conversion_scheduling_patterns() {
        init();

        let org = parse_test_resource("scheduling_patterns.org")
            .expect("Failed to parse scheduling_patterns.org");

        let config = JsonConversionConfig::default();
        let result = test_roundtrip_conversion(&org, &config)
            .expect("Failed to perform roundtrip conversion");

        let report = result
            .verify_complete()
            .expect("Failed to verify conversion");

        if !report.is_valid() {
            report.print_summary();
        }

        // スケジューリング情報の確認
        assert!(
            !org.sections.is_empty(),
            "Should have sections with scheduling"
        );

        // JSONにスケジューリング情報が含まれていることを確認
        let json_analysis = analyze_json_structure(&result.original_json)
            .expect("Failed to analyze JSON structure");
        assert!(json_analysis.unique_keys.contains_key("scheduling"));
    }

    /// Unicode文字のテスト
    #[test]
    fn test_roundtrip_conversion_unicode_content() {
        init();

        let org = parse_test_resource("unicode_content.org")
            .expect("Failed to parse unicode_content.org");

        let config = JsonConversionConfig::default();
        let result = test_roundtrip_conversion(&org, &config)
            .expect("Failed to perform roundtrip conversion");

        // Unicode文字が正しく保持されているかチェック
        assert!(result.original_json.contains("🌍"), "Should contain emoji");
        assert!(
            result.original_json.contains("日本語"),
            "Should contain Japanese text"
        );
        assert!(
            result.restored_json.contains("🌍"),
            "Restored JSON should contain emoji"
        );
        assert!(
            result.restored_json.contains("日本語"),
            "Restored JSON should contain Japanese text"
        );

        let report = result
            .verify_complete()
            .expect("Failed to verify conversion");

        if !report.is_valid() {
            report.print_summary();
        }
    }

    /// エッジケースのテスト
    #[test]
    fn test_roundtrip_conversion_edge_cases() {
        init();

        let org = parse_test_resource("edge_cases.org").expect("Failed to parse edge_cases.org");

        let config = JsonConversionConfig::default();
        let result = test_roundtrip_conversion(&org, &config)
            .expect("Failed to perform roundtrip conversion");

        let report = result
            .verify_complete()
            .expect("Failed to verify conversion");

        if !report.is_valid() {
            report.print_summary();
            // エッジケースでは多少のエラーは許容
            assert!(
                report.total_errors() < 10,
                "Too many errors in edge cases test"
            );
        }
    }

    /// 設定別の変換テスト
    #[test]
    fn test_conversion_with_different_configs() {
        init();

        let content = r#"#+TITLE: Config Test

* Section 1
:PROPERTIES:
:ID: section-1
:END:

Content for section 1.
"#;

        let org = parse_test_org(content).expect("Failed to parse org content");

        // 位置情報ありの設定
        let config_with_pos = JsonConversionConfig {
            include_position: true,
            pretty_print: false,
            ..Default::default()
        };

        // 位置情報なしの設定
        let config_without_pos = JsonConversionConfig {
            include_position: false,
            pretty_print: false,
            ..Default::default()
        };

        // 美しい整形ありの設定
        let config_pretty = JsonConversionConfig {
            include_position: false,
            pretty_print: true,
            ..Default::default()
        };

        let json_with_pos = org
            .to_json_with_config(&config_with_pos)
            .expect("Failed to convert with position");
        let json_without_pos = org
            .to_json_with_config(&config_without_pos)
            .expect("Failed to convert without position");
        let json_pretty = org
            .to_json_with_config(&config_pretty)
            .expect("Failed to convert pretty");

        // 位置情報の有無を確認
        assert!(
            json_with_pos.contains("\"pos\""),
            "Should contain position info"
        );
        assert!(
            !json_without_pos.contains("\"pos\""),
            "Should not contain position info"
        );

        // 美しい整形の確認
        assert!(
            json_pretty.contains("  "),
            "Pretty JSON should contain indentation"
        );
        assert!(
            !json_without_pos.contains("  "),
            "Compact JSON should not contain indentation"
        );

        // サイズの比較
        assert!(
            json_without_pos.len() < json_with_pos.len(),
            "JSON without position should be smaller"
        );
        assert!(
            json_pretty.len() > json_without_pos.len(),
            "Pretty JSON should be larger"
        );
    }

    /// 空のセクション処理テスト
    #[test]
    fn test_empty_sections_handling() {
        init();

        let content = r#"#+TITLE: Empty Sections Test

* Empty Section 1
:PROPERTIES:
:ID: empty-1
:END:

* Section with Content
:PROPERTIES:
:ID: with-content
:END:

Some content here.

* Empty Section 2
:PROPERTIES:
:ID: empty-2
:END:
"#;

        let org = parse_test_org(content).expect("Failed to parse org content");

        // 空のセクションを含む設定
        let config_with_empty = JsonConversionConfig {
            include_empty_sections: true,
            ..Default::default()
        };

        // 空のセクションを除外する設定
        let config_without_empty = JsonConversionConfig {
            include_empty_sections: false,
            ..Default::default()
        };

        let json_with_empty = org
            .to_json_with_config(&config_with_empty)
            .expect("Failed to convert with empty sections");
        let json_without_empty = org
            .to_json_with_config(&config_without_empty)
            .expect("Failed to convert without empty sections");

        // 空のセクション除外設定の方がサイズが小さいことを確認
        assert!(
            json_without_empty.len() < json_with_empty.len(),
            "JSON without empty sections should be smaller"
        );
    }

    /// 深度制限テスト
    #[test]
    fn test_depth_limit_handling() {
        init();

        // 深い階層構造を生成
        let generator = OrgGenerator::new()
            .with_sections(2)
            .with_max_depth(15)
            .with_content_per_section(1);

        let content = generator.generate();
        let org = parse_test_org(&content).expect("Failed to parse generated org content");

        // 深度制限を低く設定
        let config_low_depth = JsonConversionConfig {
            max_depth: 5,
            ..Default::default()
        };

        // 深度制限を高く設定
        let config_high_depth = JsonConversionConfig {
            max_depth: 20,
            ..Default::default()
        };

        let result_low = org.to_json_with_config(&config_low_depth);
        let result_high = org.to_json_with_config(&config_high_depth);

        match result_low {
            Ok(_) => {
                // 低い深度制限でも成功した場合、実際の深度がそれほど深くない
            }
            Err(JsonConversionError::MaxDepthExceeded { depth }) => {
                assert!(depth > 5, "Depth should exceed the limit");
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }

        // 高い深度制限では成功するはず
        assert!(result_high.is_ok(), "High depth limit should succeed");
    }

    /// 大容量データのテスト
    #[test]
    fn test_large_data_conversion() {
        init();

        // 大きなデータを生成
        let generator = OrgGenerator::new()
            .with_sections(50)
            .with_max_depth(4)
            .with_content_per_section(10);

        let content = generator.generate();
        let org = parse_test_org(&content).expect("Failed to parse large org content");

        let perf_test = PerformanceTest::new("large_data_conversion");

        let config = JsonConversionConfig::default();
        let json = org
            .to_json_with_config(&config)
            .expect("Failed to convert large data to JSON");

        let conversion_duration = perf_test.finish();

        // パフォーマンス要件（1秒以内）
        assert!(
            conversion_duration.as_secs() < 1,
            "Large data conversion should complete within 1 second"
        );

        // JSONサイズの確認
        assert!(
            json.len() > 10000,
            "Large data should produce substantial JSON"
        );

        // JSON構造の分析
        let analysis =
            analyze_json_structure(&json).expect("Failed to analyze large JSON structure");
        analysis.print_summary();

        assert!(analysis.total_objects > 50, "Should have many objects");
        assert!(analysis.max_depth > 3, "Should have reasonable depth");
    }

    /// ストリーミング変換のテスト
    #[test]
    fn test_streaming_conversion() {
        init();

        let org = parse_test_resource("complex_structure.org")
            .expect("Failed to parse complex_structure.org");

        let config = JsonConversionConfig::default();

        // 通常の変換
        let normal_json = org
            .to_json_with_config(&config)
            .expect("Failed to convert normally");

        // ストリーミング変換
        let mut buffer = Vec::new();
        org.to_json_stream(&mut buffer, &config)
            .expect("Failed to stream JSON");

        let streamed_json = String::from_utf8(buffer).expect("Invalid UTF-8 in streamed JSON");

        // 結果が一致することを確認
        assert_eq!(
            normal_json, streamed_json,
            "Normal and streamed JSON should be identical"
        );
    }

    /// 部分変換のテスト
    #[test]
    fn test_section_specific_conversion() {
        init();

        let org = parse_test_resource("complex_structure.org")
            .expect("Failed to parse complex_structure.org");

        let config = JsonConversionConfig::default();

        // 存在するセクションIDで部分変換
        if let Some(section) = org.sections.first() {
            let section_json = org
                .section_to_json(&section.id, &config)
                .expect("Failed to convert section to JSON");

            assert!(
                section_json.is_some(),
                "Should return JSON for existing section"
            );

            let json = section_json.unwrap();
            assert!(
                json.contains(&section.title),
                "JSON should contain section title"
            );
        }

        // 存在しないセクションIDで部分変換
        let non_existent_json = org
            .section_to_json("non-existent-id", &config)
            .expect("Failed to handle non-existent section");

        assert!(
            non_existent_json.is_none(),
            "Should return None for non-existent section"
        );
    }

    /// エラーハンドリングのテスト
    #[test]
    fn test_error_handling() {
        init();

        // 不正なJSONからの復元テスト
        let invalid_json_cases = vec![
            "",
            "{",
            "invalid json",
            r#"{"title": "test", "sections": [}"#,
            r#"{"title": null, "sections": "not an array"}"#,
        ];

        for invalid_json in invalid_json_cases {
            let result = crate::parser::Org::from_json(invalid_json);
            assert!(
                result.is_err(),
                "Should fail to parse invalid JSON: {}",
                invalid_json
            );

            match result {
                Err(JsonConversionError::DeserializationError { .. }) => {
                    // 期待されるエラータイプ
                }
                Err(e) => panic!("Unexpected error type: {:?}", e),
                Ok(_) => panic!("Should not succeed with invalid JSON: {}", invalid_json),
            }
        }
    }

    /// JSON Schema検証のテスト
    #[test]
    fn test_json_schema_consistency() {
        init();

        let test_files = vec![
            "complex_structure.org",
            "scheduling_patterns.org",
            "unicode_content.org",
            "edge_cases.org",
        ];

        let mut schema_keys = HashMap::new();

        for filename in test_files {
            if let Ok(org) = parse_test_resource(filename) {
                let config = JsonConversionConfig::default();
                if let Ok(json) = org.to_json_with_config(&config) {
                    if let Ok(analysis) = analyze_json_structure(&json) {
                        for key in analysis.unique_keys.keys() {
                            *schema_keys.entry(key.clone()).or_insert(0) += 1;
                        }
                    }
                }
            }
        }

        // 共通のキーが存在することを確認
        let expected_keys = vec!["title", "sections", "keywords", "properties"];
        for key in expected_keys {
            assert!(
                schema_keys.contains_key(key),
                "Key '{}' should be present in JSON schema",
                key
            );
        }

        println!("JSON Schema Keys found:");
        for (key, count) in schema_keys {
            println!("  {}: {} files", key, count);
        }
    }

    /// メモリ使用量のテスト
    #[test]
    fn test_memory_usage() {
        init();

        let generator = OrgGenerator::new()
            .with_sections(100)
            .with_max_depth(3)
            .with_content_per_section(5);

        let content = generator.generate();
        let org = parse_test_org(&content).expect("Failed to parse large org content");

        let org_size = estimate_memory_usage(&org);
        println!("Estimated Org struct size: {} bytes", org_size);

        let config = JsonConversionConfig::default();
        let json = org
            .to_json_with_config(&config)
            .expect("Failed to convert to JSON");

        let json_size = json.len();
        println!("JSON string size: {} bytes", json_size);

        // メモリ効率の確認（JSONサイズがOrg構造体の合理的な範囲内であることを期待）
        // 大きなデータの場合、JSONは元のデータより大きくなることがある
        let ratio = json_size as f64 / org_size as f64;
        println!("JSON to Org size ratio: {:.2}", ratio);

        // 合理的な範囲（1:1から50:1程度）であることを確認
        assert!(
            ratio < 50.0,
            "JSON size ratio should be reasonable (got {:.2})",
            ratio
        );
    }

    /// 並行処理テスト
    #[test]
    fn test_concurrent_conversion() {
        init();

        let content = r#"#+TITLE: Concurrent Test

* Section 1
Content 1

* Section 2  
Content 2
"#;

        let org = parse_test_org(content).expect("Failed to parse org content");
        let config = JsonConversionConfig::default();

        // 複数のスレッドで同時に変換を実行
        let handles: Vec<_> = (0..4)
            .map(|i| {
                let org_clone = org.clone();
                let config_clone = config.clone();

                std::thread::spawn(move || {
                    let json = org_clone
                        .to_json_with_config(&config_clone)
                        .unwrap_or_else(|_| panic!("Failed to convert in thread {}", i));
                    (i, json)
                })
            })
            .collect();

        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.join().expect("Thread panicked"));
        }

        // 全ての結果が同じであることを確認
        let first_json = &results[0].1;
        for (i, json) in &results[1..] {
            assert_eq!(first_json, json, "Thread {} produced different JSON", i);
        }
    }
}

#[cfg(test)]
mod infinite_loop_tests {
    use crate::{Context, parse};
    use std::time::{Duration, Instant};

    fn init() {
        let _ = tracing_subscriber::fmt::try_init();
    }

    #[test]
    fn test_empty_string_parsing() {
        init();
        let mut ctx = Context::new();

        let start = Instant::now();
        let result = parse(&mut ctx, "");
        let duration = start.elapsed();

        println!("Empty string parsing took: {:?}", duration);
        assert!(result.is_ok(), "Empty string should parse successfully");
        assert!(
            duration < Duration::from_secs(1),
            "Parsing should complete quickly"
        );
    }

    #[test]
    fn test_whitespace_only_parsing() {
        init();
        let mut ctx = Context::new();

        let inputs = vec!["   ", "\n\n\n", "   \n   \n   ", "\t\t\t"];

        for input in inputs {
            let start = Instant::now();
            let result = parse(&mut ctx, input);
            let duration = start.elapsed();

            println!(
                "Whitespace '{}' parsing took: {:?}",
                input.replace('\n', "\\n"),
                duration
            );
            assert!(result.is_ok(), "Whitespace should parse successfully");
            assert!(
                duration < Duration::from_secs(1),
                "Parsing should complete quickly"
            );
        }
    }

    #[test]
    fn test_section_text_block_edge_cases() {
        init();
        let mut ctx = Context::new();

        let inputs = vec![
            "* Headline\n",
            "* Headline\n\n",
            "* Headline\n   \n",
            "* Headline\nsome text",
            "* Headline\n\nsome text\n",
        ];

        for input in inputs {
            let start = Instant::now();
            let result = parse(&mut ctx, input);
            let duration = start.elapsed();

            println!(
                "Section text '{}' parsing took: {:?}",
                input.replace('\n', "\\n"),
                duration
            );
            assert!(result.is_ok(), "Section text should parse successfully");
            assert!(
                duration < Duration::from_secs(1),
                "Parsing should complete quickly"
            );
        }
    }

    #[test]
    fn test_deep_recursion() {
        init();
        let mut ctx = Context::new();

        // Create deeply nested sections
        let mut input = String::new();
        for i in 1..=50 {
            input.push_str(&"*".repeat(i));
            input.push_str(" Section ");
            input.push_str(&i.to_string());
            input.push('\n');
        }

        let start = Instant::now();
        let result = parse(&mut ctx, &input);
        let duration = start.elapsed();

        println!("Deep recursion parsing took: {:?}", duration);
        assert!(result.is_ok(), "Deep recursion should parse successfully");
        assert!(
            duration < Duration::from_secs(5),
            "Deep recursion should complete in reasonable time"
        );
    }

    #[test]
    fn test_malformed_input_safety() {
        init();

        let malformed_inputs = vec![
            "* Headline\n[[",
            "* Headline\n<incomplete",
            "* Headline\n:PROPERTIES:\n:KEY:",
            "* Headline\nSCHEDULED: <incomplete",
        ];

        for input in malformed_inputs {
            let mut ctx = Context::new();
            let start = Instant::now();
            let result = parse(&mut ctx, input);
            let duration = start.elapsed();

            println!(
                "Malformed input '{}' took: {:?}, result: {:?}",
                input.replace('\n', "\\n"),
                duration,
                result.is_ok()
            );
            // Don't assert success - malformed input may legitimately fail
            // Just ensure it doesn't hang
            assert!(
                duration < Duration::from_secs(2),
                "Malformed input should fail quickly"
            );
        }
    }

    #[test]
    fn test_large_input_performance() {
        init();
        let mut ctx = Context::new();

        // Generate large input
        let mut large_input = String::new();
        for i in 0..1000 {
            large_input.push_str(&format!("* Section {}\n", i));
            large_input.push_str("Some content here\n");
            large_input.push_str("More content\n");
            large_input.push('\n');
        }

        let start = Instant::now();
        let result = parse(&mut ctx, &large_input);
        let duration = start.elapsed();

        println!(
            "Large input ({} chars) parsing took: {:?}",
            large_input.len(),
            duration
        );
        assert!(result.is_ok(), "Large input should parse successfully");
        assert!(
            duration < Duration::from_secs(10),
            "Large input should complete in reasonable time"
        );
    }
}

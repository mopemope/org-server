#[cfg(test)]
mod stress_tests {
    use crate::{Context, parse};
    use std::time::{Duration, Instant};

    fn init() {
        let _ = tracing_subscriber::fmt::try_init();
    }

    #[test]
    fn test_section_text_block_stress() {
        init();
        let mut ctx = Context::new();

        // Test cases that might trigger infinite loops in section_text_block
        let problematic_inputs = vec![
            // Empty section content
            "* Headline\n\n* Next",
            // Section with only whitespace
            "* Headline\n   \n* Next",
            // Section with newlines only
            "* Headline\n\n\n\n* Next",
            // Mixed whitespace and newlines
            "* Headline\n \t \n  \n\t\n* Next",
        ];

        for input in problematic_inputs {
            let start = Instant::now();
            let result = parse(&mut ctx, input);
            let duration = start.elapsed();

            println!(
                "Stress test '{}' took: {:?}",
                input.replace('\n', "\\n"),
                duration
            );
            assert!(result.is_ok(), "Should parse successfully");
            assert!(
                duration < Duration::from_millis(100),
                "Should complete very quickly"
            );
        }
    }

    #[test]
    fn test_pest_rule_directly() {
        init();
        use crate::parser::{OrgParser, Rule};
        use pest::Parser;

        // Test section_text_block rule directly
        let test_cases = vec!["", "   ", "\n", "text", "text with spaces"];

        for input in test_cases {
            let start = Instant::now();
            let result = OrgParser::parse(Rule::section_text_block, input);
            let duration = start.elapsed();

            println!(
                "Direct rule test '{}' took: {:?}, success: {}",
                input.replace('\n', "\\n"),
                duration,
                result.is_ok()
            );
            assert!(
                duration < Duration::from_millis(10),
                "Rule should complete very quickly"
            );
        }
    }

    #[test]
    fn test_edge_case_combinations() {
        init();
        let mut ctx = Context::new();

        // Combinations that might cause issues
        let edge_cases = vec![
            // Incomplete hyperlinks
            "* Test\n[[incomplete",
            "* Test\n[[link][incomplete",
            // Incomplete time stamps
            "* Test\n<2023-12-",
            "* Test\n[2023-12-",
            // Mixed incomplete elements
            "* Test\n[[link\n<2023-12-\nSCHEDULED:",
        ];

        for input in edge_cases {
            let start = Instant::now();
            let _result = parse(&mut ctx, input);
            let duration = start.elapsed();

            println!(
                "Edge case '{}' took: {:?}",
                input.replace('\n', "\\n"),
                duration
            );
            // Don't assert success - these may legitimately fail
            assert!(
                duration < Duration::from_millis(500),
                "Should fail quickly if it fails"
            );
        }
    }
}

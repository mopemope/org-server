use pest::Parser;
use pest::Span;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "org.pest"]
pub struct OrgParser;

#[cfg(test)]
mod tests {
    use super::*;
    use pest::Parser;
    use tracing::debug;

    fn init() {
        let _ = tracing_subscriber::fmt::try_init();
    }

    #[test]
    fn parse_header() {
        init();
        let pairs = OrgParser::parse(Rule::header, "** 日本語").unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            for inner_pair in pair.into_inner() {
                let s = inner_pair.as_str();
                match inner_pair.as_rule() {
                    Rule::header_symbol => {
                        assert_eq!("**", s);
                    }
                    Rule::header_title => {
                        assert_eq!("日本語", s);
                    }
                    _ => {}
                }
            }
        }
    }
}

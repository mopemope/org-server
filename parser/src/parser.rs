use anyhow::Result;
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use tracing::debug;

#[derive(Parser)]
#[grammar = "org.pest"]
pub struct OrgParser;

#[derive(Clone, Default)]
pub struct Context {}

#[derive(Clone, Debug, Default)]
pub struct Org {
    filename: Option<String>,
    id: Option<String>,
    title: Option<String>,
    properties: Vec<Property>,
    keywords: Vec<Keyword>,
    sections: Vec<Section>,
}

impl Org {
    fn new() -> Self {
        Org {
            filename: None,
            id: None,
            title: None,
            properties: Vec::new(),
            keywords: Vec::new(),
            sections: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Keyword {
    key: String,
    value: String,
}

impl Keyword {
    fn new(key: &str, value: &str) -> Self {
        Keyword {
            key: key.to_string(),
            value: value.to_string(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Property {
    key: String,
    value: String,
}

impl Property {
    fn new(key: &str, value: &str) -> Self {
        Property {
            key: key.to_string(),
            value: value.to_string(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Section {
    title: String,
    properties: Vec<Property>,
    keywords: Vec<Keyword>,
    contents: Option<String>,

    sections: Vec<Section>,
}

fn parse_property(pair: Pair<'_, Rule>) -> Property {
    let mut prop: Property = Default::default();

    for pair in pair.into_inner() {
        match pair.as_rule() {
            Rule::property_key => {
                prop.key = pair.as_str().to_string();
            }
            Rule::property_value => {
                prop.value = pair.as_str().to_string();
            }
            _ => {}
        }
    }
    prop
}

fn parse_keyword(pair: Pair<'_, Rule>) -> Keyword {
    let mut kw: Keyword = Default::default();

    for pair in pair.into_inner() {
        match pair.as_rule() {
            Rule::keyword_key => {
                kw.key = pair.as_str().to_string();
            }
            Rule::keyword_value => {
                kw.value = pair.as_str().to_string();
            }
            _ => {
                debug!("## {:?}", pair);
            }
        }
    }
    kw
}

fn parse_section(pair: Pair<'_, Rule>) -> Section {
    let mut section: Section = Default::default();
    for pair in pair.into_inner() {
        match pair.as_rule() {
            Rule::headline => {
                for pair in pair.into_inner() {
                    match pair.as_rule() {
                        Rule::headline_symbol => {}
                        Rule::headline_title => {
                            section.title = pair.as_str().to_string();
                        }
                        _ => {}
                    }
                }
            }
            Rule::properties => {
                for pair in pair.into_inner() {
                    let prop = parse_property(pair);
                    section.properties.push(prop);
                }
            }
            Rule::keyword => {
                let kw = parse_keyword(pair);
                section.keywords.push(kw);
            }
            Rule::content => {
                section.contents = Some(pair.as_str().to_string());
            }
            _ => {}
        }
    }

    return section;
}

pub fn parse(content: &str) -> Result<Org> {
    let mut org = Org::default();
    let mut pairs = OrgParser::parse(Rule::org, content)?;
    if let Some(pair) = pairs.next() {
        for pair in pair.into_inner() {
            match pair.as_rule() {
                Rule::properties => {
                    for pair in pair.into_inner() {
                        let prop = parse_property(pair);
                        org.properties.push(prop);
                    }
                }
                Rule::keyword => {
                    let kw = parse_keyword(pair);
                    if kw.key.to_lowercase() == "title" {
                        org.title = Some(kw.value.to_string());
                    }
                    org.keywords.push(kw);
                }
                Rule::section => {
                    let sec = parse_section(pair);
                    org.sections.push(sec);
                }
                _ => {
                    debug!("! {:?}", pair);
                }
            }
        }
    }

    Ok(org)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pest::Parser;

    fn init() {
        let _ = tracing_subscriber::fmt::try_init();
    }

    #[test]
    fn test_rule_active_quote() {
        init();
        let content = "<2023-12-11 Mon 07:09>";
        let pairs =
            OrgParser::parse(Rule::active_quoted, content).unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            println!("{:?}", pair);
        }
    }

    #[test]
    fn test_rule_inactive_quote() {
        init();
        let content = "[2023-12-11 Mon 07:09]";
        let pairs =
            OrgParser::parse(Rule::inactive_quoted, content).unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            println!("{:?}", pair);
        }
    }

    #[test]
    fn test_rule_headline() {
        init();
        let pairs = OrgParser::parse(Rule::headline, "** 日 本 語  :abc:def:")
            .unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            for inner_pair in pair.into_inner() {
                let s = inner_pair.as_str();
                match inner_pair.as_rule() {
                    Rule::headline_symbol => {
                        assert_eq!("**", s);
                    }
                    Rule::headline_title => {
                        assert_eq!("日 本 語  ", s);
                    }
                    Rule::tags => {
                        for (i, inner_pair) in inner_pair.into_inner().enumerate() {
                            match i {
                                0 => {
                                    assert_eq!("abc", inner_pair.as_str());
                                }
                                1 => {
                                    assert_eq!("def", inner_pair.as_str());
                                }
                                _ => {
                                    debug!("{:?}", inner_pair);
                                }
                            }
                        }
                    }
                    _ => {
                        println!("{:?}", inner_pair);
                    }
                }
            }
        }
    }

    // drawer tests
    #[test]
    fn test_rule_property_start() {
        init();
        let content = r#":PROPERTIES:"#;
        let pairs =
            OrgParser::parse(Rule::property_start, content).unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            debug!("{:?}", pair);
        }
        let content = r#":properties:"#;
        let pairs =
            OrgParser::parse(Rule::property_start, content).unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            debug!("{:?}", pair);
        }
    }

    #[test]
    fn test_rule_property_end() {
        init();
        let content = r#":END:"#;
        let pairs =
            OrgParser::parse(Rule::property_end, content).unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            debug!("{:?}", pair);
        }
        let content = r#":end:"#;
        let pairs =
            OrgParser::parse(Rule::property_end, content).unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            debug!("{:?}", pair);
        }
    }

    #[test]
    fn test_rule_property() {
        init();

        let content = r#":ID:   :value   "#;
        let pairs = OrgParser::parse(Rule::property, content).unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            for pair in pair.into_inner() {
                match pair.as_rule() {
                    Rule::property_key => {
                        assert_eq!("ID", pair.as_str());
                    }
                    Rule::property_value => {
                        assert_eq!(":value", pair.as_str());
                    }
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn test_rule_property_timestamp() {
        init();
        let content = ":CREATED:    <2023-12-11 Mon 07:09>";
        let pairs = OrgParser::parse(Rule::property, content).unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            for pair in pair.into_inner() {
                match pair.as_rule() {
                    Rule::property_key => {
                        assert_eq!("CREATED", pair.as_str());
                    }
                    Rule::property_value => {
                        assert_eq!("<2023-12-11 Mon 07:09>", pair.as_str());
                    }
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn test_rule_properties() {
        init();

        let content = r#":PROPERTIES:
:ID:   :value
:ID:     :value
:END:
"#;
        let pairs = OrgParser::parse(Rule::properties, content).unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            for pair in pair.into_inner() {
                for pair in pair.into_inner() {
                    match pair.as_rule() {
                        Rule::property_key => {
                            assert_eq!("ID", pair.as_str());
                        }
                        Rule::property_value => {
                            assert_eq!(":value", pair.as_str());
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    #[test]
    fn test_rule_drawer_start() {
        init();
        let content = ":LOGBOOK:";
        let pairs =
            OrgParser::parse(Rule::drawer_start, content).unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            debug!("{:?}", pair);
        }
        let content = ":logbook:";
        let pairs =
            OrgParser::parse(Rule::drawer_start, content).unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            debug!("{:?}", pair);
        }
    }

    #[test]
    fn test_rule_drawer_end() {
        init();
        let content = ":END:";
        let pairs = OrgParser::parse(Rule::drawer_end, content).unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            debug!("{:?}", pair);
        }
        let content = ":end:";
        let pairs = OrgParser::parse(Rule::drawer_end, content).unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            debug!("{:?}", pair);
        }
    }

    #[test]
    fn test_rule_drawer_contents() {
        init();
        let content = " <2023-12-26 Tue 08:02> [2023-12-26 Tue 09:02] abc def";
        let pairs =
            OrgParser::parse(Rule::drawer_contents, content).unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            for (i, pair) in pair.into_inner().enumerate() {
                match pair.as_rule() {
                    Rule::active_quoted => {
                        assert_eq!(0, i);
                        assert_eq!("<2023-12-26 Tue 08:02>", pair.as_str());
                    }
                    Rule::inactive_quoted => {
                        assert_eq!(1, i);
                        assert_eq!("[2023-12-26 Tue 09:02]", pair.as_str());
                    }
                    Rule::drawer_content => {
                        assert_eq!(2, i);
                        assert_eq!("abc def", pair.as_str());
                    }
                    _ => {
                        debug!("*** {:?}", pair);
                    }
                }
            }
        }
    }

    #[test]
    fn test_rule_drawer_all() {
        init();

        let content = r#":LOGBOOK:
[1 abc def] :abc:
:END:
"#;
        let pairs = OrgParser::parse(Rule::drawer, content).unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            for pair in pair.into_inner() {
                // debug!("** {:?}", pair);
                for pair in pair.into_inner() {
                    match pair.as_rule() {
                        Rule::drawer_content => {
                            assert_eq!(":abc:", pair.as_str());
                        }
                        Rule::inactive_quoted => {
                            assert_eq!("[1 abc def]", pair.as_str());
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // keyword tests
    #[test]
    fn test_rule_keyword() {
        init();

        let content = "#+TODO: TODO(t) | DONE(d)";
        let pairs = OrgParser::parse(Rule::keyword, content).unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            for pair in pair.into_inner() {
                match pair.as_rule() {
                    Rule::keyword_key => {
                        assert_eq!("TODO", pair.as_str());
                    }
                    Rule::keyword_value => {
                        assert_eq!("TODO(t) | DONE(d)", pair.as_str());
                    }
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn test_rule_options_keyword() {
        init();

        let content = "#+OPTIONS: ^:nil";
        let pairs = OrgParser::parse(Rule::keyword, content).unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            for pair in pair.into_inner() {
                match pair.as_rule() {
                    Rule::keyword_key => {
                        assert_eq!("OPTIONS", pair.as_str());
                    }
                    Rule::keyword_value => {
                        assert_eq!("^:nil", pair.as_str());
                    }
                    _ => {}
                }
            }
        }
    }

    // tags tests
    #[test]
    fn test_rule_tags() {
        init();

        let content = ":abc:def:";
        let pairs = OrgParser::parse(Rule::tags, content).unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            for (i, pair) in pair.into_inner().enumerate() {
                match pair.as_rule() {
                    Rule::tag => match i {
                        0 => {
                            assert_eq!("abc", pair.as_str());
                        }
                        1 => {
                            assert_eq!("def", pair.as_str());
                        }
                        _ => {
                            println!("{:?} {:?}", i, pair);
                        }
                    },
                    _ => {
                        println!("{:?} ", pair);
                    }
                }
            }
        }
    }

    // section tests
    #[test]
    fn test_rule_section() {
        init();

        let content = r#"* TEST
:PROPERTIES:
:ID:   :value
:END:

Content
"#;
        let pairs = OrgParser::parse(Rule::section, content).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            for pair in pair.into_inner() {
                match pair.as_rule() {
                    Rule::headline => {
                        for pair in pair.into_inner() {
                            match pair.as_rule() {
                                Rule::headline_symbol => {
                                    assert_eq!("*", pair.as_str());
                                }
                                Rule::headline_title => {
                                    assert_eq!("TEST", pair.as_str());
                                }
                                _ => {
                                    println!("!!! {:?}", pair);
                                }
                            }
                        }
                    }
                    Rule::content => {
                        assert_eq!("Content\n", pair.as_str());
                    }
                    Rule::properties => {
                        for pair in pair.into_inner() {
                            for pair in pair.into_inner() {
                                match pair.as_rule() {
                                    Rule::property_key => {
                                        assert_eq!("ID", pair.as_str());
                                    }
                                    Rule::property_value => {
                                        assert_eq!(":value", pair.as_str());
                                    }
                                    _ => {
                                        println!("!!! {:?}", pair);
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        println!("{:?}", pair);
                    }
                }
            }
        }
    }

    // org tests
    #[test]
    fn test_rule_org() {
        init();

        let content = r#":PROPERTIES:
:ID:   value
:END:
#+TITLE: title

* TEST1
:PROPERTIES:
:ID:   value1
:CREATED: <2023-12-26 Tue 08:02>
:END:
Content1

* test2
:PROPERTIES:
:ID:   value2
:END:
Content2

"#;
        let pairs = OrgParser::parse(Rule::org, content).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            for (i, pair) in pair.into_inner().enumerate() {
                match pair.as_rule() {
                    Rule::properties => {
                        for pair in pair.into_inner() {
                            for pair in pair.into_inner() {
                                match pair.as_rule() {
                                    Rule::property_key => {
                                        assert_eq!("ID", pair.as_str());
                                    }
                                    Rule::property_value => {
                                        assert_eq!("value", pair.as_str());
                                    }
                                    _ => todo!(),
                                }
                            }
                        }
                    }
                    Rule::keyword => {
                        for pair in pair.into_inner() {
                            match pair.as_rule() {
                                Rule::keyword_key => {
                                    assert_eq!("TITLE", pair.as_str());
                                }
                                Rule::keyword_value => {
                                    assert_eq!("title", pair.as_str());
                                }
                                _ => {
                                    todo!();
                                }
                            }
                        }
                    }
                    Rule::section => match i {
                        2 => {
                            // * TEST1
                            // :PROPERTIES:
                            // :ID:   value1
                            // :CREATED: <2023-12-26 Tue 08:02>
                            // :END:
                            // Content1

                            for pair in pair.into_inner() {
                                match pair.as_rule() {
                                    Rule::headline => {
                                        for pair in pair.into_inner() {
                                            match pair.as_rule() {
                                                Rule::headline_symbol => {
                                                    assert_eq!("*", pair.as_str());
                                                }
                                                Rule::headline_title => {
                                                    assert_eq!("TEST1", pair.as_str());
                                                }
                                                _ => {
                                                    todo!();
                                                }
                                            }
                                        }
                                    }
                                    Rule::content => {
                                        assert_eq!("Content1\n\n", pair.as_str());
                                    }
                                    Rule::properties => {
                                        for (i, pair) in pair.into_inner().enumerate() {
                                            for pair in pair.into_inner() {
                                                match i {
                                                    0 => match pair.as_rule() {
                                                        Rule::property_key => {
                                                            assert_eq!("ID", pair.as_str());
                                                        }
                                                        Rule::property_value => {
                                                            assert_eq!("value1", pair.as_str());
                                                        }
                                                        _ => todo!(),
                                                    },
                                                    1 => match pair.as_rule() {
                                                        Rule::property_key => {
                                                            assert_eq!("CREATED", pair.as_str());
                                                        }
                                                        Rule::property_value => {
                                                            assert_eq!(
                                                                "<2023-12-26 Tue 08:02>",
                                                                pair.as_str()
                                                            );
                                                        }
                                                        _ => todo!(),
                                                    },
                                                    _ => todo!(),
                                                }
                                            }
                                        }
                                    }

                                    _ => {
                                        todo!();
                                    }
                                }
                            }
                        }
                        3 => {
                            for pair in pair.into_inner() {
                                match pair.as_rule() {
                                    Rule::headline => {
                                        for pair in pair.into_inner() {
                                            match pair.as_rule() {
                                                Rule::headline_symbol => {
                                                    assert_eq!("*", pair.as_str());
                                                }
                                                Rule::headline_title => {
                                                    assert_eq!("test2", pair.as_str());
                                                }
                                                _ => {
                                                    todo!();
                                                }
                                            }
                                        }
                                    }
                                    Rule::content => {
                                        assert_eq!("Content2\n\n", pair.as_str());
                                    }
                                    Rule::properties => {
                                        for (i, pair) in pair.into_inner().enumerate() {
                                            for pair in pair.into_inner() {
                                                match i {
                                                    0 => match pair.as_rule() {
                                                        Rule::property_key => {
                                                            assert_eq!("ID", pair.as_str());
                                                        }
                                                        Rule::property_value => {
                                                            assert_eq!("value2", pair.as_str());
                                                        }
                                                        _ => {
                                                            todo!();
                                                        }
                                                    },
                                                    _ => todo!(),
                                                }
                                            }
                                        }
                                    }

                                    _ => {
                                        todo!();
                                    }
                                }
                            }
                        }
                        _ => {
                            todo!();
                        }
                    },
                    _ => {
                        todo!();
                    }
                }
            }
        }
    }

    #[test]
    fn test_rule_org_simple1() {
        init();

        let content = r#":PROPERTIES:
:ID:   value
:END:
#+TITLE: title

"#;
        let pairs = OrgParser::parse(Rule::org, content).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            for pair in pair.into_inner() {
                println!("{:?}", pair);
            }
        }
    }

    #[test]
    fn test_parse_org() {
        init();

        let content = r#":PROPERTIES:
:ID:   value
:END:
#+TITLE: title
#+STARTUP: overview

* SECTION 1
#+KEYWORD1: title1
:PROPERTIES:
:ID: 461e7f4a-5467-4e1b-baed-517a02c00b9c
:CREATED: <2024-01-02 Tue 12:34>
:END:
#+KEYWORD2: title2
CONTENT1
CONTENT1

"#;
        let org = parse(content).unwrap_or_else(|e| panic!("{}", e));

        debug!("{:?}", org);

        assert_eq!(1, org.properties.len());
        assert_eq!(2, org.keywords.len());

        assert_eq!(1, org.sections.len());
    }
}

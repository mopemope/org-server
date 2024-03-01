use anyhow::Result;
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tracing::debug;

#[derive(Parser)]
#[grammar = "org.pest"]
pub struct OrgParser;

#[derive(Clone, Default)]
pub struct Context {}

impl Context {
    fn new() -> Self {
        Context {}
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Org {
    filename: Option<String>,
    id: Option<String>,
    title: Option<String>,
    drawers: Vec<Drawer>,
    properties: Vec<Properties>,
    keywords: Vec<Keyword>,
    sections: Vec<Section>,
}

impl Org {
    fn new() -> Self {
        Org {
            filename: None,
            id: None,
            title: None,
            drawers: Vec::new(),
            properties: Vec::new(),
            keywords: Vec::new(),
            sections: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Keyword {
    key: String,
    value: String,
    col: usize,
    line: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Properties {
    col: usize,
    line: usize,
    children: Vec<Property>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Property {
    key: String,
    value: String,
    col: usize,
    line: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Drawer {
    name: String,
    col: usize,
    line: usize,
    children: Vec<Content>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Content {
    col: usize,
    line: usize,
    contents: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Section {
    col: usize,
    line: usize,
    title: String,
    drawers: Vec<Drawer>,
    properties: Vec<Properties>,
    keywords: Vec<Keyword>,
    contents: Vec<Content>,
    sections: Vec<Section>,
}

fn parse_properties(ctx: &mut Context, pair: Pair<'_, Rule>) -> Properties {
    let mut properties: Properties = Default::default();
    let (line, col) = pair.line_col();
    properties.line = line;
    properties.col = col;

    for pair in pair.into_inner() {
        let mut prop: Property = Default::default();

        for pair in pair.into_inner() {
            match pair.as_rule() {
                Rule::property_key => {
                    let (line, col) = pair.line_col();
                    prop.key = pair.as_str().to_string();
                    prop.line = line;
                    prop.col = col;
                }
                Rule::property_value => {
                    prop.value = pair.as_str().to_string();
                }
                _ => {}
            }
        }
        properties.children.push(prop);
    }
    properties
}

fn parse_drawer(ctx: &mut Context, pair: Pair<'_, Rule>) -> Drawer {
    let mut drawer: Drawer = Default::default();
    let (line, col) = pair.line_col();
    drawer.line = line;
    drawer.col = col;

    for pair in pair.into_inner() {
        match pair.as_rule() {
            Rule::drawer_name => {
                drawer.name = pair.as_str().to_string();
            }
            Rule::drawer_contents => {
                for pair in pair.into_inner() {
                    match pair.as_rule() {
                        Rule::drawer_content => {
                            let mut content: Content = Default::default();
                            let (line, col) = pair.line_col();
                            content.line = line;
                            content.col = col;
                            content.contents = pair.as_str().to_string();
                            drawer.children.push(content);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    drawer
}

fn parse_keyword(ctx: &mut Context, pair: Pair<'_, Rule>) -> Keyword {
    let mut kw: Keyword = Default::default();

    for pair in pair.into_inner() {
        match pair.as_rule() {
            Rule::keyword_key => {
                let (line, col) = pair.line_col();
                kw.key = pair.as_str().to_string();
                kw.col = col;
                kw.line = line;
            }
            Rule::keyword_value => {
                kw.value = pair.as_str().to_string();
            }
            _ => {
                todo!();
            }
        }
    }
    kw
}

fn parse_section(ctx: &mut Context, pair: Pair<'_, Rule>) -> Section {
    let mut section: Section = Default::default();
    let (line, col) = pair.line_col();
    section.col = col;
    section.line = line;

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
                let prop = parse_properties(ctx, pair);
                section.properties.push(prop);
            }
            Rule::drawer => {
                let drawer = parse_drawer(ctx, pair);
                section.drawers.push(drawer);
            }
            Rule::keyword => {
                let kw = parse_keyword(ctx, pair);
                section.keywords.push(kw);
            }
            Rule::content => {
                let mut content: Content = Default::default();
                let (line, col) = pair.line_col();
                content.col = col;
                content.line = line;
                content.contents = pair.as_str().to_string();
                section.contents.push(content);
            }
            Rule::section => {
                let sec = parse_section(ctx, pair);
                section.sections.push(sec);
            }
            _ => {}
        }
    }

    section
}

pub fn parse(ctx: &mut Context, content: &str) -> Result<Org> {
    let mut org = Org::default();
    let mut pairs = OrgParser::parse(Rule::org, content)?;
    if let Some(pair) = pairs.next() {
        for pair in pair.into_inner() {
            match pair.as_rule() {
                Rule::properties => {
                    let props = parse_properties(ctx, pair);
                    for prop in &props.children {
                        if prop.key.to_lowercase() == "id" {
                            org.id = Some(prop.value.clone());
                        }
                    }
                    org.properties.push(props);
                }
                Rule::drawer => {
                    let drawer = parse_drawer(ctx, pair);
                    org.drawers.push(drawer);
                }
                Rule::keyword => {
                    let kw = parse_keyword(ctx, pair);
                    if kw.key.to_lowercase() == "title" {
                        org.title = Some(kw.value.to_string());
                    }
                    org.keywords.push(kw);
                }
                Rule::section => {
                    let sec = parse_section(ctx, pair);
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

pub fn parse_file(ctx: &mut Context, path: &Path) -> Result<Org> {
    let content = fs::read_to_string(path)?;
    let mut org = parse(ctx, content.as_str())?;
    org.filename = Some(path.display().to_string());
    Ok(org)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pest::Parser;
    use std::path::PathBuf;

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
        debug!("{:?}", pairs.len());
        assert!(pairs.len() > 0);
        for pair in pairs {
            match pair.as_rule() {
                Rule::drawer_start => {
                    let name = pair.as_str();
                    debug!("{:?}", name);
                }
                _ => {
                    debug!("{:?}", pair.as_rule());
                }
            }
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
            let pairs = pair.into_inner();
            assert!(pairs.len() > 0);
            for pair in pairs {
                // debug!("** {:?}", pair);
                match pair.as_rule() {
                    Rule::drawer_name => {
                        assert_eq!("LOGBOOK", pair.as_str());
                    }
                    Rule::drawer_contents => {
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
                    _ => {}
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
:LOGBOOK:
CLOCK: [2024-02-27 Tue 09:56]--[2024-02-27 Tue 17:56] =>  8:00
:END:
#+KEYWORD2: title2
CONTENT1
CONTENT1

"#;

        let mut ctx = Context::new();
        let org = parse(&mut ctx, content).unwrap_or_else(|e| panic!("{}", e));

        debug!("{:?}", org);

        assert_eq!(1, org.properties.len());
        assert_eq!(2, org.keywords.len());
        assert_eq!(1, org.sections.len());

        let sec = org.sections.first().unwrap();

        assert_eq!(1, sec.drawers.len());
    }

    #[test]
    fn test_parse_file() -> Result<()> {
        init();
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("tests/resources/test-1.org");

        let mut ctx = Context::new();
        let org = parse_file(&mut ctx, &d)?;

        let sec = &org.sections[1];
        // debug!("{:?}", org);
        debug!("{:?}", &sec.contents);
        Ok(())
    }
}

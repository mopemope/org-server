use crate::{reminder::get_reminders, Reminder};
use anyhow::Result;
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use tracing::debug;

#[derive(Parser)]
#[grammar = "org.pest"]
pub struct OrgParser;

#[derive(Clone, Default)]
pub struct Context {} // TODO add attr

impl Context {
    pub fn new() -> Self {
        Context {}
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Org {
    pub filename: Option<String>,
    pub id: Option<String>,
    pub title: Option<String>,
    pub drawers: Vec<Drawer>,
    pub properties: Vec<Properties>,
    pub keywords: Vec<Keyword>,
    pub sections: Vec<Section>,
}

impl Org {
    pub fn new() -> Self {
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

    pub fn get_reminders(&self) -> Vec<Reminder> {
        let mut res = vec![];
        for sec in &self.sections {
            let mut reminders = get_reminders(sec);
            if !reminders.is_empty() {
                res.append(&mut reminders)
            }
        }
        res
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Keyword {
    pub key: String,
    pub value: String,
    pub col: usize,
    pub line: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Properties {
    pub col: usize,
    pub line: usize,
    pub children: Vec<Property>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Property {
    pub key: String,
    pub value: String,
    pub col: usize,
    pub line: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Drawer {
    pub name: String,
    pub col: usize,
    pub line: usize,
    pub children: Vec<Content>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Content {
    pub col: usize,
    pub line: usize,
    pub contents: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Section {
    pub col: usize,
    pub line: usize,
    pub title: String,
    pub drawers: Vec<Drawer>,
    pub properties: Vec<Properties>,
    pub keywords: Vec<Keyword>,
    pub contents: Vec<Content>,
    pub sections: Vec<Section>,
    pub scheduling: Vec<Scheduling>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq)]
pub enum Scheduling {
    Scheduled(String),
    Deadline(String),
}

impl PartialEq for Scheduling {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Scheduling::Scheduled(a), Scheduling::Scheduled(b)) => a == b,
            (Scheduling::Deadline(a), Scheduling::Deadline(b)) => a == b,
            _ => false,
        }
    }
}

impl Hash for Scheduling {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Scheduling::Scheduled(data) => {
                state.write(&[1]);
                data.hash(state);
            }
            Scheduling::Deadline(data) => {
                state.write(&[2]);
                data.hash(state);
            }
        }
    }
}

fn parse_properties(_ctx: &mut Context, pair: Pair<'_, Rule>) -> Properties {
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

fn parse_drawer(_ctx: &mut Context, pair: Pair<'_, Rule>) -> Drawer {
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

fn parse_keyword(_ctx: &mut Context, pair: Pair<'_, Rule>) -> Keyword {
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
                let hl = pair.as_str();
                if let Ok(parsed) = OrgParser::parse(Rule::headline, hl) {
                    for pair in parsed {
                        for pair in pair.into_inner() {
                            match pair.as_rule() {
                                Rule::headline_symbol => {}
                                Rule::headline_title => {
                                    section.title = pair.as_str().to_string();
                                }
                                Rule::tags => {
                                    // TODO
                                }
                                _ => {}
                            }
                        }
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
            Rule::scheduling => {
                for pair in pair.into_inner() {
                    match pair.as_rule() {
                        Rule::scheduled => {
                            if let Some(pair) = pair.into_inner().next() {
                                if let Some(pair) = pair.into_inner().next() {
                                    let sch = Scheduling::Scheduled(pair.as_str().to_string());
                                    section.scheduling.push(sch);
                                }
                            }
                        }
                        Rule::deadline => {
                            if let Some(pair) = pair.into_inner().next() {
                                if let Some(pair) = pair.into_inner().next() {
                                    let sch = Scheduling::Deadline(pair.as_str().to_string());
                                    section.scheduling.push(sch);
                                }
                            }
                        }
                        _ => {}
                    }
                }
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
            _ => {
                debug!("FIXME {:?}", pair);
            }
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
                    let sec = parse_section(ctx, pair); // TODO parse src block
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
    fn test_rule_active_time_quote() {
        init();
        let content = "<2023-12-11 Mon 07:09>";
        let pairs =
            OrgParser::parse(Rule::active_time_quoted, content).unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            println!("{:?}", pair);
        }
    }

    #[test]
    fn test_rule_scheduled() {
        init();
        let content = "SCHEDULED: <2023-12-11 Mon 07:09>";
        let pairs = OrgParser::parse(Rule::scheduled, content).unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            assert_eq!(Rule::scheduled, pair.as_rule());
            for pair in pair.into_inner() {
                assert_eq!(Rule::active_time_quoted, pair.as_rule());
                // println!("{:?}", pair);
            }
        }
    }

    #[test]
    fn test_rule_inactive_time_quote() {
        init();
        let content = "[2023-12-11 Mon 07:09]";
        let pairs = OrgParser::parse(Rule::inactive_time_quoted, content)
            .unwrap_or_else(|e| panic!("{}", e));
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
        let pairs = OrgParser::parse(Rule::headline, "** TODO 日 本 語  :abc:")
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
                    Rule::active_time_quoted => {
                        assert_eq!(0, i);
                        assert_eq!("<2023-12-26 Tue 08:02>", pair.as_str());
                    }
                    Rule::inactive_time_quoted => {
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
                                Rule::inactive_time_quoted => {
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
SCHEDULED: <2024-12-03 Tue 12:34>
DEADLINE: <2024-12-03 Tue 10:30>
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

        // debug!("{:?}", org);

        assert_eq!(1, org.properties.len());
        assert_eq!(2, org.keywords.len());
        assert_eq!(1, org.sections.len());

        let sec = org.sections.first().unwrap();

        assert_eq!(1, sec.drawers.len());

        let rems = org.get_reminders();
        assert_eq!(6, rems.len());
    }

    #[test]
    fn test_parse_file() -> Result<()> {
        init();
        let mut d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("tests/resources/test-1.org");

        let content = std::fs::read_to_string(&d)?;
        let mut ctx = Context::new();

        let org = parse(&mut ctx, &content)?;

        let _sec = &org.sections[1];
        // debug!("{:?}", org);
        // debug!("{:?}", &sec.contents);

        let result = serde_json::to_string(&org)?;
        debug!("{:?}", result);

        Ok(())
    }
}

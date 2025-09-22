use crate::display;
use crate::{Reminder, reminder::get_reminders};
use anyhow::Result;
use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use tracing::debug;
use uuid::Uuid;

#[derive(Parser)]
#[grammar = "org.pest"]
pub struct OrgParser;

#[derive(Clone, Default)]
pub struct Context {} // TODO add attr

impl Context {
    #[must_use]
    pub const fn new() -> Self {
        Self {}
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
    #[must_use]
    pub const fn new() -> Self {
        Self {
            filename: None,
            id: None,
            title: None,
            drawers: Vec::new(),
            properties: Vec::new(),
            keywords: Vec::new(),
            sections: Vec::new(),
        }
    }

    #[must_use]
    pub fn get_reminders(&self) -> Vec<Reminder> {
        let mut res = vec![];
        for sec in &self.sections {
            let mut reminders = get_reminders(sec);
            if !reminders.is_empty() {
                res.append(&mut reminders);
            }
        }
        res
    }

    #[must_use]
    pub fn get_hyperlinks(&self) -> Vec<(String, Option<String>)> {
        let mut res = vec![];
        for sec in &self.sections {
            let mut links = sec.get_hyperlinks();
            if !links.is_empty() {
                res.append(&mut links);
            }
        }
        res
    }

    #[must_use]
    pub const fn display(&self) -> display::OrgDisplay<'_> {
        display::OrgDisplay { inner: self }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Eq)]
pub struct Pos {
    pub col: usize,
    pub line: usize,
}

impl Pos {
    pub const fn new(col: usize, line: usize) -> Self {
        Self { col, line }
    }
}

impl PartialEq for Pos {
    fn eq(&self, other: &Self) -> bool {
        self.col == other.col && self.line == other.line
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Keyword {
    pub pos: Pos,
    pub key: String,
    pub value: String,
}

impl Keyword {
    pub const fn display(&self) -> display::KeywordDisplay<'_> {
        display::KeywordDisplay { inner: self }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Properties {
    pub pos: Pos,
    pub children: Vec<Property>,
}

impl Properties {
    pub const fn display(&self) -> display::PropertiesDisplay<'_> {
        display::PropertiesDisplay { inner: self }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Property {
    pub pos: Pos,
    pub key: String,
    pub value: String,
}

impl Property {
    pub const fn display(&self) -> display::PropertyDisplay<'_> {
        display::PropertyDisplay { inner: self }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Drawer {
    pub pos: Pos,
    pub name: String,
    pub children: Vec<Row>,
}

impl Drawer {
    pub const fn display(&self) -> display::DrawerDisplay<'_> {
        display::DrawerDisplay { inner: self }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq)]
pub enum Content {
    Text(Pos, String),
    Hyperlink(Pos, String, Option<String>),
}

impl Content {
    pub const fn display(&self) -> display::ContentDisplay<'_> {
        display::ContentDisplay { inner: self }
    }
}

impl PartialEq for Content {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Text(_, a), Self::Text(_, b)) => a == b,
            (Self::Hyperlink(_, link1, a), Self::Hyperlink(_, link2, b)) => {
                link1 == link2 && a == b
            }
            _ => false,
        }
    }
}

impl Hash for Content {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Text(_, text) => {
                state.write(&[1]);
                state.write(text.as_bytes());
                text.hash(state);
            }
            Self::Hyperlink(_, link, desc) => {
                state.write(&[2]);
                state.write(link.as_bytes());
                desc.hash(state);
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Row {
    pub pos: Pos,
    pub contents: Vec<Content>,
}

impl Row {
    pub const fn display(&self) -> display::RowDisplay<'_> {
        display::RowDisplay { inner: self }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Section {
    pub pos: Pos,
    pub id: String,
    pub headline_symbol: String,
    pub title: String,
    pub drawers: Vec<Drawer>,
    pub properties: Vec<Properties>,
    pub keywords: Vec<Keyword>,
    pub contents: Vec<Row>,
    pub sections: Vec<Section>,
    pub scheduling: Vec<Scheduling>,
}

impl Section {
    pub fn get_hyperlinks(&self) -> Vec<(String, Option<String>)> {
        let mut res = vec![];
        for row in &self.contents {
            for content in &row.contents {
                if let Content::Hyperlink(_, link, desc) = content {
                    res.push((link.to_owned(), desc.clone()));
                }
            }
        }
        res
    }

    pub const fn display(&self) -> display::SectionDisplay<'_> {
        display::SectionDisplay { inner: self }
    }
}

impl Default for Section {
    fn default() -> Self {
        Self {
            pos: Pos::default(),
            id: Uuid::new_v4().to_string(),
            headline_symbol: String::default(),
            title: String::default(),
            drawers: Vec::default(),
            properties: Vec::default(),
            keywords: Vec::default(),
            contents: Vec::default(),
            sections: Vec::default(),
            scheduling: Vec::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq)]
pub enum Scheduling {
    Scheduled(Pos, String, String),
    Deadline(Pos, String, String),
}

impl Scheduling {
    pub const fn display(&self) -> display::SchedulingDisplay<'_> {
        display::SchedulingDisplay { inner: self }
    }
}

impl PartialEq for Scheduling {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Scheduled(_, t1, a), Self::Scheduled(_, t2, b)) => t1 == t2 && a == b,
            (Self::Deadline(_, t1, a), Self::Deadline(_, t2, b)) => t1 == t2 && a == b,
            _ => false,
        }
    }
}

impl Hash for Scheduling {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Scheduled(_, title, data) => {
                state.write(&[1]);
                state.write(title.as_bytes());
                data.hash(state);
            }
            Self::Deadline(_, title, data) => {
                state.write(&[2]);
                state.write(title.as_bytes());
                data.hash(state);
            }
        }
    }
}

fn parse_properties(_ctx: &mut Context, pair: Pair<'_, Rule>) -> Properties {
    let mut properties: Properties = Properties::default();
    let (line, col) = pair.line_col();
    properties.pos = Pos::new(col, line);

    for pair in pair.into_inner() {
        let mut prop: Property = Property::default();

        for pair in pair.into_inner() {
            match pair.as_rule() {
                Rule::property_key => {
                    let (line, col) = pair.line_col();
                    prop.key = pair.as_str().to_string();
                    prop.pos = Pos::new(col, line);
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
    let mut drawer: Drawer = Drawer::default();
    let (line, col) = pair.line_col();
    drawer.pos = Pos::new(col, line);

    for pair in pair.into_inner() {
        match pair.as_rule() {
            Rule::drawer_name => {
                drawer.name = pair.as_str().to_string();
            }
            Rule::drawer_contents => {
                for pair in pair.into_inner() {
                    if pair.as_rule() == Rule::drawer_content {
                        let mut row: Row = Row::default();
                        let (line, col) = pair.line_col();
                        row.pos = Pos::new(col, line);
                        // TODO text only ?
                        row.contents.push(Content::Text(
                            Pos::new(col, line),
                            pair.as_str().to_string(),
                        ));
                        drawer.children.push(row);
                    }
                }
            }
            _ => {}
        }
    }
    drawer
}

fn parse_keyword(_ctx: &mut Context, pair: Pair<'_, Rule>) -> Keyword {
    let mut kw: Keyword = Keyword::default();

    for pair in pair.into_inner() {
        match pair.as_rule() {
            Rule::keyword_key => {
                let (line, col) = pair.line_col();
                kw.key = pair.as_str().to_string();
                kw.pos = Pos::new(col, line);
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

#[allow(clippy::too_many_lines)]
fn parse_section(ctx: &mut Context, pair: Pair<'_, Rule>) -> Section {
    let mut section: Section = Section::default();
    let (line, col) = pair.line_col();
    section.pos = Pos::new(col, line);

    for pair in pair.into_inner() {
        match pair.as_rule() {
            Rule::headline => {
                let hl = pair.as_str();
                if let Ok(parsed) = OrgParser::parse(Rule::headline, hl) {
                    for pair in parsed {
                        for pair in pair.into_inner() {
                            match pair.as_rule() {
                                Rule::headline_symbol => {
                                    section.headline_symbol = pair.as_str().to_string();
                                }
                                Rule::headline_title => {
                                    section.title = pair.as_str().to_string();
                                }
                                _ => {
                                    // TODO: Handle tags and other rules
                                }
                            }
                        }
                    }
                }
            }
            Rule::properties => {
                let props = parse_properties(ctx, pair);
                for prop in &props.children {
                    if prop.key.to_lowercase() == "id" {
                        section.id.clone_from(&prop.value);
                    }
                }
                section.properties.push(props);
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
                            if let Some(pair) = pair.into_inner().next()
                                && let Some(pair) = pair.into_inner().next()
                            {
                                let (line, col) = pair.line_col();
                                let pos = Pos::new(col, line);
                                let sch = Scheduling::Scheduled(
                                    pos,
                                    section.title.clone(),
                                    pair.as_str().to_string(),
                                );
                                section.scheduling.push(sch);
                            }
                        }
                        Rule::deadline => {
                            if let Some(pair) = pair.into_inner().next()
                                && let Some(pair) = pair.into_inner().next()
                            {
                                let (line, col) = pair.line_col();
                                let pos = Pos::new(col, line);
                                let sch = Scheduling::Deadline(
                                    pos,
                                    section.title.clone(),
                                    pair.as_str().to_string(),
                                );
                                section.scheduling.push(sch);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Rule::section_text_block => {
                let mut row: Row = Row::default();
                let (line, col) = pair.line_col();
                row.pos = Pos::new(col, line);
                for pair in pair.into_inner() {
                    let (line, col) = pair.line_col();
                    let pos = Pos::new(col, line);
                    match pair.as_rule() {
                        Rule::section_text => {
                            let text = Content::Text(pos, pair.as_str().to_owned());
                            row.contents.push(text);
                        }
                        Rule::section_link => {
                            if let Some(pair) = pair.into_inner().next() {
                                let mut pairs = pair.into_inner();
                                if let Some(link) = pairs.next() {
                                    let description = pairs.next().map(|p| p.as_str().to_string());
                                    let hyperlink = Content::Hyperlink(
                                        pos,
                                        link.as_str().to_string(),
                                        description,
                                    );
                                    row.contents.push(hyperlink);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                section.contents.push(row);
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

/// Parse org-mode content into an Org structure
///
/// # Errors
///
/// Returns an error if the content cannot be parsed according to org-mode syntax rules
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
            println!("{pair:?}");
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
            println!("{pair:?}");
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
                        println!("{inner_pair:?}");
                    }
                }
            }
        }
    }

    // drawer tests
    #[test]
    fn test_rule_property_start() {
        init();
        let content = r":PROPERTIES:";
        let pairs =
            OrgParser::parse(Rule::property_start, content).unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            debug!("{:?}", pair);
        }
        let content = r":properties:";
        let pairs =
            OrgParser::parse(Rule::property_start, content).unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            debug!("{:?}", pair);
        }
    }

    #[test]
    fn test_rule_property_end() {
        init();
        let content = r":END:";
        let pairs =
            OrgParser::parse(Rule::property_end, content).unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            debug!("{:?}", pair);
        }
        let content = r":end:";
        let pairs =
            OrgParser::parse(Rule::property_end, content).unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            debug!("{:?}", pair);
        }
    }

    #[test]
    fn test_rule_property() {
        init();

        let content = r":ID:   :value   ";
        let pairs = OrgParser::parse(Rule::property, content).unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            for pair in pair.into_inner() {
                match pair.as_rule() {
                    Rule::property_key => {
                        assert_eq!("ID", pair.as_str());
                    }
                    Rule::property_value => {
                        assert_eq!(":value   ", pair.as_str());
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

        let content = r":PROPERTIES:
:ID:   :value
:ID:     :value
:END:
";
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

        let content = r":LOGBOOK:
[1 abc def] :abc:
:END:
";
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
                            println!("{i:?} {pair:?}");
                        }
                    },
                    _ => {
                        println!("{pair:?} ");
                    }
                }
            }
        }
    }

    // section tests
    #[test]
    fn test_rule_section() {
        init();

        let content = r"* TEST
:PROPERTIES:
:ID:   :value1
:END:

ABCDEF

";
        let pairs = OrgParser::parse(Rule::section, content).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            for (i, pair) in pair.into_inner().enumerate() {
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
                                    // println!("!!! {:?}", pair);
                                    todo!()
                                }
                            }
                        }
                    }
                    Rule::section_text_block => {
                        if i == 2 {
                            assert_eq!("ABCDEF", pair.as_str());
                        } else if i == 3 {
                            assert_eq!("", pair.as_str());
                        }
                    }
                    Rule::properties => {
                        for pair in pair.into_inner() {
                            for pair in pair.into_inner() {
                                match pair.as_rule() {
                                    Rule::property_key => {
                                        assert_eq!("ID", pair.as_str());
                                    }
                                    Rule::property_value => {
                                        assert_eq!(":value1", pair.as_str());
                                    }
                                    _ => {
                                        debug!("!!! {:?}", pair);
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        debug!("! {:?}", pair);
                    }
                }
            }
        }
    }

    // org tests
    #[test]
    fn test_rule_org() {
        init();

        let content = r":PROPERTIES:
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

";
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

                            for (i, pair) in pair.into_inner().enumerate() {
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
                                    Rule::section_text_block => {
                                        if i == 2 {
                                            assert_eq!("Content1", pair.as_str());
                                        } else if i == 3 {
                                            assert_eq!("", pair.as_str());
                                        }
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
                            for (i, pair) in pair.into_inner().enumerate() {
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
                                    Rule::section_text_block => {
                                        if i == 2 {
                                            assert_eq!("Content2", pair.as_str());
                                        } else if i == 3 {
                                            assert_eq!("", pair.as_str());
                                        }
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
    fn test_parse_hyperlink() {
        init();

        let content = r"

* Section
TEST1
[[https://example.com]] TEST2
[[https://example.com][Dectription]] TEST3
TEST4
";

        let mut ctx = Context::new();
        let _org = parse(&mut ctx, content).unwrap_or_else(|e| panic!("{}", e));
    }

    #[test]
    fn test_rule_org_simple1() {
        init();

        let content = r":PROPERTIES:
:ID:   value
:END:
#+TITLE: title

";
        let pairs = OrgParser::parse(Rule::org, content).unwrap_or_else(|e| panic!("{}", e));
        for pair in pairs {
            for _pair in pair.into_inner() {
                // debug!("{:?}", pair);
            }
        }
    }

    #[test]
    fn test_parse_org() {
        init();

        let content = r":PROPERTIES:
:ID:   value
:END:
#+TITLE: title
#+STARTUP: overview

* SECTION 1
SCHEDULED: <2025-12-03 Wed 12:34>
DEADLINE: <2025-12-03 Wed 10:30>
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
CONTENT2

* SECTION 2

";

        let mut ctx = Context::new();
        let org = parse(&mut ctx, content).unwrap_or_else(|e| panic!("{}", e));

        // debug!("{:?}", org);

        assert_eq!(1, org.properties.len());
        assert_eq!(2, org.keywords.len());
        assert_eq!(2, org.sections.len());

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
        let links = org.get_hyperlinks();
        debug!("{:?}", links);

        let result = serde_json::to_string(&org)?;
        debug!("{:?}", result);

        Ok(())
    }
}

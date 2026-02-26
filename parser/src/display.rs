use crate::parser;

pub trait Display: std::fmt::Display {
    fn line(&self) -> usize;
}

pub struct PropertyDisplay<'a> {
    pub inner: &'a parser::Property,
}

impl std::fmt::Display for PropertyDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, ":{}: {}", self.inner.key, self.inner.value)?;
        Ok(())
    }
}

impl Display for PropertyDisplay<'_> {
    fn line(&self) -> usize {
        self.inner.pos.line
    }
}

pub struct PropertiesDisplay<'a> {
    pub inner: &'a parser::Properties,
}

impl std::fmt::Display for PropertiesDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut props: Vec<PropertyDisplay> =
            self.inner.children.iter().map(|p| p.display()).collect();
        props.sort_by_key(Display::line);
        writeln!(f, ":PROPERTIES:")?;
        for p in props {
            writeln!(f, "{p}")?;
        }
        write!(f, ":END:")?;
        Ok(())
    }
}

impl Display for PropertiesDisplay<'_> {
    fn line(&self) -> usize {
        self.inner.pos.line
    }
}

pub struct KeywordDisplay<'a> {
    pub inner: &'a parser::Keyword,
}

impl std::fmt::Display for KeywordDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "#+{}: {}", self.inner.key, self.inner.value)?;
        Ok(())
    }
}

impl Display for KeywordDisplay<'_> {
    fn line(&self) -> usize {
        self.inner.pos.line
    }
}

pub struct ContentDisplay<'a> {
    pub inner: &'a parser::Content,
}

impl std::fmt::Display for ContentDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let str = match self.inner {
            parser::Content::Text(_, text) => text.to_owned(),
            parser::Content::Hyperlink(_, link, desc) => desc.as_ref().map_or_else(
                || format!("[[{link}]]"),
                |desc| format!("[[{link}][{desc}]]"),
            ),
        };
        write!(f, "{str}")?;
        Ok(())
    }
}

impl Display for ContentDisplay<'_> {
    fn line(&self) -> usize {
        let pos = match self.inner {
            parser::Content::Text(pos, _) | parser::Content::Hyperlink(pos, _, _) => pos,
        };
        pos.line
    }
}

pub struct RowDisplay<'a> {
    pub inner: &'a parser::Row,
}

impl std::fmt::Display for RowDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for c in &self.inner.contents {
            write!(f, "{}", c.display())?;
        }
        Ok(())
    }
}

impl Display for RowDisplay<'_> {
    fn line(&self) -> usize {
        self.inner.pos.line
    }
}

pub struct DrawerDisplay<'a> {
    pub inner: &'a parser::Drawer,
}

impl std::fmt::Display for DrawerDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut contents: Vec<RowDisplay> =
            self.inner.children.iter().map(|p| p.display()).collect();
        contents.sort_by_key(Display::line);

        writeln!(f, ":{}:", self.inner.name.to_uppercase())?;
        for c in contents {
            writeln!(f, "{c}")?;
        }

        write!(f, ":END:")?;

        Ok(())
    }
}

impl Display for DrawerDisplay<'_> {
    fn line(&self) -> usize {
        self.inner.pos.line
    }
}

pub struct SchedulingDisplay<'a> {
    pub inner: &'a parser::Scheduling,
}

impl std::fmt::Display for SchedulingDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let str = match self.inner {
            parser::Scheduling::Scheduled(_, _, dt) => format!("SCHEDULED: <{dt}>"),
            parser::Scheduling::Deadline(_, _, dt) => format!("DEADLINE: <{dt}>"),
        };
        write!(f, "{str}")?;
        Ok(())
    }
}

impl Display for SchedulingDisplay<'_> {
    fn line(&self) -> usize {
        let pos = match self.inner {
            parser::Scheduling::Scheduled(pos, _, _) | parser::Scheduling::Deadline(pos, _, _) => {
                pos
            }
        };
        pos.line
    }
}

pub struct PlainListDisplay<'a> {
    pub inner: &'a parser::PlainList,
}

impl std::fmt::Display for PlainListDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for (i, item) in self.inner.items.iter().enumerate() {
            let indent = " ".repeat(item.indent);
            write!(f, "{}{}", indent, item.bullet)?;
            if let Some(ref cb) = item.checkbox {
                let cb_str = match cb {
                    parser::CheckboxState::Unchecked => "[ ]",
                    parser::CheckboxState::Checked => "[X]",
                    parser::CheckboxState::Partial => "[-]",
                };
                write!(f, " {cb_str}")?;
            }
            if let Some(ref term) = item.description_term {
                write!(f, " {term} ::")?;
            }
            write!(f, " {}", item.text)?;
            if i < self.inner.items.len() - 1 {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

impl Display for PlainListDisplay<'_> {
    fn line(&self) -> usize {
        self.inner.pos.line
    }
}

pub struct TableDisplay<'a> {
    pub inner: &'a parser::Table,
}

impl std::fmt::Display for TableDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for (i, row) in self.inner.rows.iter().enumerate() {
            match row {
                parser::TableRow::Standard(cells) => {
                    write!(f, "|")?;
                    for cell in cells {
                        write!(f, " {} |", cell)?;
                    }
                }
                parser::TableRow::Rule => {
                    // Fallback to generic hrule, since original spacing isn't preserved
                    // in this basic representation.
                    write!(f, "|---|")?;
                }
            }
            if i < self.inner.rows.len() - 1 {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

impl Display for TableDisplay<'_> {
    fn line(&self) -> usize {
        self.inner.pos.line
    }
}

pub struct SectionDisplay<'a> {
    pub inner: &'a parser::Section,
}

impl std::fmt::Display for SectionDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut contents: Vec<Box<dyn Display>> = vec![];

        for kw in &self.inner.keywords {
            contents.push(Box::new(kw.display()));
        }
        for sch in &self.inner.scheduling {
            contents.push(Box::new(sch.display()));
        }
        for props in &self.inner.properties {
            contents.push(Box::new(props.display()));
        }
        for drawer in &self.inner.drawers {
            contents.push(Box::new(drawer.display()));
        }
        for con in &self.inner.contents {
            contents.push(Box::new(con.display()));
        }
        for list in &self.inner.lists {
            contents.push(Box::new(list.display()));
        }
        for table in &self.inner.tables {
            contents.push(Box::new(table.display()));
        }
        for sec in &self.inner.sections {
            contents.push(Box::new(sec.display()));
        }

        contents.sort_by_key(|a| a.line());

        // headline: symbol + TODO + priority + title + tags
        write!(f, "{}", self.inner.headline_symbol)?;
        if let Some(ref status) = self.inner.todo_status {
            write!(f, " {status}")?;
        }
        if let Some(ref priority) = self.inner.priority {
            write!(f, " [#{priority}]")?;
        }
        write!(f, " {}", self.inner.title)?;
        if !self.inner.tags.is_empty() {
            write!(f, " :{}:", self.inner.tags.join(":"))?;
        }
        writeln!(f)?;

        let mut line = self.line() + 1;
        for c in contents {
            let start = c.line();

            if line == start {
                let buf = c.to_string();
                for s in buf.split('\n') {
                    writeln!(f, "{s}")?;
                    line += 1;
                }
            } else {
                while line != start {
                    writeln!(f)?;
                    line += 1;
                }
                let buf = c.to_string();
                for s in buf.split('\n') {
                    writeln!(f, "{s}")?;
                    line += 1;
                }
            }
        }

        // code_blocks don't have a Display trait impl, output directly
        for cb in &self.inner.code_blocks {
            if cb.language.is_empty() {
                writeln!(f, "#+BEGIN_SRC")?;
            } else {
                writeln!(f, "#+BEGIN_SRC {}", cb.language)?;
            }
            if !cb.body.is_empty() {
                writeln!(f, "{}", cb.body)?;
            }
            writeln!(f, "#+END_SRC")?;
        }

        Ok(())
    }
}

impl Display for SectionDisplay<'_> {
    fn line(&self) -> usize {
        self.inner.pos.line
    }
}

pub struct OrgDisplay<'a> {
    pub inner: &'a parser::Org,
}

impl std::fmt::Display for OrgDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut contents: Vec<Box<dyn Display>> = vec![];

        for kw in &self.inner.keywords {
            contents.push(Box::new(kw.display()));
        }
        for props in &self.inner.properties {
            contents.push(Box::new(props.display()));
        }
        for drawer in &self.inner.drawers {
            contents.push(Box::new(drawer.display()));
        }
        for sec in &self.inner.sections {
            contents.push(Box::new(sec.display()));
        }
        contents.sort_by_key(|a| a.line());

        let mut line = 1;
        for c in contents {
            let start = c.line();
            if line == start {
                let buf = c.to_string();
                for s in buf.split('\n') {
                    writeln!(f, "{s}")?;
                    line += 1;
                }
            } else {
                while line != start {
                    writeln!(f)?;
                    line += 1;
                }
                let buf = c.to_string();
                for s in buf.split('\n') {
                    writeln!(f, "{s}")?;
                    line += 1;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use crate::parser::{Context, Org, parse};
    use tracing::debug;

    fn init() {
        let _ = tracing_subscriber::fmt::try_init();
    }

    fn get_test_org() -> Org {
        let content = r":PROPERTIES:
:ID:   value1
:ID:   value2
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
CONTENT2

";

        let mut ctx = Context::new();
        parse(&mut ctx, content).unwrap_or_else(|e| panic!("{}", e))
    }

    #[test]
    fn test_display_org() {
        init();
        let org = get_test_org();
        debug!("{:?}", org.display().to_string());
        // std::fs::write("/tmp/foo.org", org.display().to_string());
    }

    #[test]
    fn test_display_scheduling() {
        init();
        let org = get_test_org();
        let sec = org.sections.first().unwrap();
        for sch in &sec.scheduling {
            debug!("{:?}", sch.display().to_string());
        }
    }

    #[test]
    fn test_display_section() {
        init();
        let org = get_test_org();
        let sec = org.sections.first().unwrap();
        debug!("{:?}", sec.display().to_string());
    }

    #[test]
    fn test_display_drawer() {
        init();
        let org = get_test_org();
        let sec = org.sections.first().unwrap();
        for drawer in &sec.drawers {
            debug!("{:?}", drawer.display().to_string());
        }
    }

    #[test]
    fn test_display_keyword() {
        init();
        let org = get_test_org();
        for kw in &org.keywords {
            debug!("{:?}", kw.display().to_string());
        }
    }

    #[test]
    fn test_display_properties() {
        init();
        let org = get_test_org();
        for props in &org.properties {
            debug!("{:?}", props.display().to_string());
        }
    }

    #[test]
    fn test_display_proprety() {
        init();
        let org = get_test_org();
        for props in &org.properties {
            for prop in &props.children {
                debug!("{:?}", prop.display().to_string());
            }
        }
    }
}

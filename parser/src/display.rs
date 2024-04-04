use crate::parser::{Keyword, Properties, Property};

pub trait HasPos {
    fn line(&self) -> usize;
}

pub struct PropertyDisplay<'a> {
    pub inner: &'a Property,
}

impl std::fmt::Display for PropertyDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, ":{}: {}", self.inner.key, self.inner.value)?;
        Ok(())
    }
}

impl HasPos for PropertyDisplay<'_> {
    fn line(&self) -> usize {
        self.inner.pos.line
    }
}

pub struct PropertiesDisplay<'a> {
    pub inner: &'a Properties,
}

impl std::fmt::Display for PropertiesDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut props: Vec<PropertyDisplay> =
            self.inner.children.iter().map(|p| p.display()).collect();
        props.sort_by_key(|a| a.line());
        writeln!(f, ":PROPERTIES:")?;
        for p in props {
            writeln!(f, "{}", p)?;
        }
        writeln!(f, ":END:")?;
        Ok(())
    }
}

impl HasPos for PropertiesDisplay<'_> {
    fn line(&self) -> usize {
        self.inner.pos.line
    }
}

pub struct KeywordDisplay<'a> {
    pub inner: &'a Keyword,
}

impl std::fmt::Display for KeywordDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // #+TITLE: 2024-04-04-Thu
        write!(f, "#+{}: {}", self.inner.key, self.inner.value)?;
        Ok(())
    }
}

impl HasPos for KeywordDisplay<'_> {
    fn line(&self) -> usize {
        self.inner.pos.line
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{parse, Context, Org};
    use tracing::debug;

    fn init() {
        let _ = tracing_subscriber::fmt::try_init();
    }

    fn get_test_org() -> Org {
        let content = r#":PROPERTIES:
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
CONTENT1

"#;

        let mut ctx = Context::new();
        let org = parse(&mut ctx, content).unwrap_or_else(|e| panic!("{}", e));
        org
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

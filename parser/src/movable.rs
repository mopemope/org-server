use crate::parser::{Content, Drawer, Keyword, Properties, Property, Section};

pub trait Movable {
    fn move_point(&mut self, col: isize, line: isize);
}

impl Movable for Content {
    fn move_point(&mut self, col: isize, line: isize) {
        self.col = self.col.saturating_add_signed(col);
        self.line = self.col.saturating_add_signed(line);
    }
}

impl Movable for Keyword {
    fn move_point(&mut self, col: isize, line: isize) {
        self.col = self.col.saturating_add_signed(col);
        self.line = self.col.saturating_add_signed(line);
    }
}

impl Movable for Properties {
    fn move_point(&mut self, col: isize, line: isize) {
        self.col = self.col.saturating_add_signed(col);
        self.line = self.col.saturating_add_signed(line);
        for prop in self.children.iter_mut() {
            prop.move_point(col, line);
        }
    }
}

impl Movable for Property {
    fn move_point(&mut self, col: isize, line: isize) {
        self.col = self.col.saturating_add_signed(col);
        self.line = self.col.saturating_add_signed(line);
    }
}

impl Movable for Drawer {
    fn move_point(&mut self, col: isize, line: isize) {
        self.col = self.col.saturating_add_signed(col);
        self.line = self.col.saturating_add_signed(line);
        for content in self.children.iter_mut() {
            content.move_point(col, line);
        }
    }
}

impl Movable for Section {
    fn move_point(&mut self, col: isize, line: isize) {
        self.col = self.col.saturating_add_signed(col);
        self.line = self.col.saturating_add_signed(line);
        for dr in self.drawers.iter_mut() {
            dr.move_point(col, line);
        }
        for prop in self.properties.iter_mut() {
            prop.move_point(col, line);
        }
        for kw in self.keywords.iter_mut() {
            kw.move_point(col, line);
        }
        for content in self.contents.iter_mut() {
            content.move_point(col, line);
        }
        for sec in self.sections.iter_mut() {
            sec.move_point(col, line);
        }
    }
}

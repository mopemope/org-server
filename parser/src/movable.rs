use crate::parser::{Content, Drawer, Keyword, Properties, Property, Scheduling, Section};

pub trait Movable {
    fn move_point(&mut self, col: isize, line: isize);
}

impl Movable for Content {
    fn move_point(&mut self, col: isize, line: isize) {
        self.pos.col = self.pos.col.saturating_add_signed(col);
        self.pos.line = self.pos.col.saturating_add_signed(line);
    }
}

impl Movable for Keyword {
    fn move_point(&mut self, col: isize, line: isize) {
        self.pos.col = self.pos.col.saturating_add_signed(col);
        self.pos.line = self.pos.col.saturating_add_signed(line);
    }
}

impl Movable for Properties {
    fn move_point(&mut self, col: isize, line: isize) {
        self.pos.col = self.pos.col.saturating_add_signed(col);
        self.pos.line = self.pos.col.saturating_add_signed(line);
        for prop in self.children.iter_mut() {
            prop.move_point(col, line);
        }
    }
}

impl Movable for Property {
    fn move_point(&mut self, col: isize, line: isize) {
        self.pos.col = self.pos.col.saturating_add_signed(col);
        self.pos.line = self.pos.col.saturating_add_signed(line);
    }
}

impl Movable for Drawer {
    fn move_point(&mut self, col: isize, line: isize) {
        self.pos.col = self.pos.col.saturating_add_signed(col);
        self.pos.line = self.pos.col.saturating_add_signed(line);
        for content in self.children.iter_mut() {
            content.move_point(col, line);
        }
    }
}

impl Movable for Scheduling {
    fn move_point(&mut self, col: isize, line: isize) {
        match self {
            Scheduling::Scheduled(ref mut pos, _) => {
                pos.col = pos.col.saturating_add_signed(col);
                pos.line = pos.col.saturating_add_signed(line);
            }
            Scheduling::Deadline(pos, _) => {
                pos.col = pos.col.saturating_add_signed(col);
                pos.line = pos.col.saturating_add_signed(line);
            }
        }
    }
}

impl Movable for Section {
    fn move_point(&mut self, col: isize, line: isize) {
        self.pos.col = self.pos.col.saturating_add_signed(col);
        self.pos.line = self.pos.col.saturating_add_signed(line);
        for dr in self.drawers.iter_mut() {
            dr.move_point(col, line);
        }
        for prop in self.properties.iter_mut() {
            prop.move_point(col, line);
        }
        for kw in self.keywords.iter_mut() {
            kw.move_point(col, line);
        }
        for sch in self.scheduling.iter_mut() {
            sch.move_point(col, line);
        }
        for content in self.contents.iter_mut() {
            content.move_point(col, line);
        }
        for sec in self.sections.iter_mut() {
            sec.move_point(col, line);
        }
    }
}

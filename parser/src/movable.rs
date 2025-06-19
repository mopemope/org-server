use crate::parser::{Drawer, Keyword, Properties, Property, Row, Scheduling, Section};

pub trait Movable {
    fn move_point(&mut self, col: isize, line: isize);
}

impl Movable for Row {
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
        for prop in &mut self.children {
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
        for content in &mut self.children {
            content.move_point(col, line);
        }
    }
}

impl Movable for Scheduling {
    fn move_point(&mut self, col: isize, line: isize) {
        match self {
            Self::Scheduled(pos, _, _) | Self::Deadline(pos, _, _) => {
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
        for dr in &mut self.drawers {
            dr.move_point(col, line);
        }
        for prop in &mut self.properties {
            prop.move_point(col, line);
        }
        for kw in &mut self.keywords {
            kw.move_point(col, line);
        }
        for sch in &mut self.scheduling {
            sch.move_point(col, line);
        }
        for content in &mut self.contents {
            content.move_point(col, line);
        }
        for sec in &mut self.sections {
            sec.move_point(col, line);
        }
    }
}

use crate::color::Color;
use crate::cursor::Cursor;
use crate::grid::rendered::RenderedLines;
use crate::grid::*;
use crate::neovim::{ModeInfo, RedrawEvent, HighlightAttr, RgbAttr};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Editor {
    lines: Lines,
    nvim_busy: bool,
    current_mode: usize,
    cursor: Cursor,
    hl_groups: HighlightGroups,
    modes: Vec<ModeInfo>,
}

impl Editor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn render(&mut self) -> RenderedLines {
        self.lines.render(&self.hl_groups)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum EventRes {
    NextOne,
    Render,
    Destroy,
    Resize(u16, u16),
}

#[derive(Debug, Default)]
pub struct HighlightGroups {
    groups: HashMap<usize, RgbAttr>,
    default: RgbAttr,
}

impl HighlightGroups {
    pub fn update_default(&mut self, default: RgbAttr) {
        self.default = default;
    }

    pub fn update(&mut self, hl_attrs: Vec<HighlightAttr>) {
        self.groups.reserve(hl_attrs.len());

        for hl in hl_attrs {
            self.groups.insert(hl.id, hl.rgb_attr);
        }
    }

    pub fn group_color_set(&self, hl_id: usize) -> RgbAttr {
        if let Some(mut hl) = self.groups.get(&hl_id) {
            hl.foreground = hl.foreground.or(self.default.foreground);
            hl.background = hl.background.or(self.default.background);
            hl.special = hl.special.or(self.default.special);

            hl
        } else {
            self.default
        }
    }
}

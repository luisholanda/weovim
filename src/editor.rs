use crate::cursor::Cursor;
use crate::grid::*;
use crate::nvim::events::grid::{RgbAttr, HighlightAttr};
use crate::nvim::events::{ModeInfo, RedrawEvent};
use crate::ui::Color;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Editor {
    lines: Lines,
    nvim_busy: bool,
    current_mode: usize,
    cursor: Cursor,
    hl_groups: HighlightGroups,
    moodes: Vec<ModeInfo>,
}

impl Editor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle_nvim_redraw_event(&mut self, event: RedrawEvent) -> EventRes {
        match event {
            RedrawEvent::ModeInfoSet { mode_info, .. } => self.moodes = mode_info,
            RedrawEvent::ModeChange { index, .. } => {
                self.current_mode = index;
                self.cursor = self.cursor.change_shape(self.moodes[index].cursor_shape);
            }
            RedrawEvent::Busy(busy) => self.nvim_busy = busy,
            RedrawEvent::Flush => return EventRes::Render,
            RedrawEvent::DefaultColorSet { fg, bg, sp } => {
                self.hl_groups.update_default(RgbAttr {
                    foreground: Some(fg),
                    background: Some(bg),
                    special: Some(sp),
                    ..RgbAttr::default()
                })
            }
            RedrawEvent::HlAttrDefine(hls) => self.hl_groups.update(hls),
            RedrawEvent::GridLine(lines) => self.lines.update_lines(lines),
            RedrawEvent::GridClear => self.lines.clear(),
            RedrawEvent::GridDestroy => return EventRes::Destroy,
            RedrawEvent::GridResize { width, height, .. } => self.lines.resize(height, width),
            RedrawEvent::GridCursorGoto(goto) => {
                self.cursor = self.cursor.move_to(goto.row, goto.column)
            }
            RedrawEvent::GridScroll(scl) => {
                let reg = [scl.top, scl.bottom, scl.left, scl.right];

                self.lines.scroll(reg, scl.rows);
            }
            e => println!("Ignoring event {:?}", e),
        }

        EventRes::NextOne
    }

    pub fn render(&mut self) {
        let text = self.lines.render(&self.hl_groups);

        for l in text {
            println!("{}", l.text);
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ColorSet {
    pub foreground: Color,
    pub background: Color,
    pub special: Color,
}

#[derive(Debug, Copy, Clone)]
pub enum EventRes {
    NextOne,
    Render,
    Destroy,
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

    pub fn group(&self, hl_id: usize) -> &RgbAttr {
        self.groups.get(&hl_id).unwrap_or(&self.default)
    }

    pub fn update(&mut self, hl_attrs: Vec<HighlightAttr>) {
        self.groups.reserve(hl_attrs.len());

        for hl in hl_attrs {
            self.groups.insert(hl.id, hl.rgb_attr);
        }
    }
}
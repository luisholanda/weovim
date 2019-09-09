use crate::cursor::CursorShape;
use crate::ui::Color;

pub mod grid;
pub mod parse;

#[derive(Debug)]
pub enum RedrawEvent {
    // Global events.
    SetTitle(String),
    ModeInfoSet {
        cursor_style_enabled: bool,
        mode_info: Vec<ModeInfo>,
    },
    OptionSet(Vec<UiOption>),
    ModeChange {
        name: String,
        index: usize,
    },
    Busy(bool),
    Flush,

    // Grid Events
    GridResize {
        grid: usize,
        width: usize,
        height: usize,
    },
    DefaultColorSet {
        fg: Color,
        bg: Color,
        sp: Color,
    },
    HlAttrDefine(Vec<grid::HighlightAttr>),
    GridLine(Vec<grid::GridLine>),
    GridClear,
    GridDestroy,
    GridCursorGoto(grid::GridGoto),
    GridScroll(grid::GridScroll),
}

#[derive(Debug, Default)]
pub struct ModeInfo {
    pub cursor_shape: CursorShape,
    pub blink_on: u64,
    pub blink_off: u64,
    pub blink_wait: u64,
    pub cell_percentage: f64,
    pub name: String,
    pub short_name: String,
}

#[derive(Debug)]
pub enum UiOption {
    GuiFont(String),
    GuiFontWide(String),
    LineSpace(i64),
}

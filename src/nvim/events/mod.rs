pub mod parse;
pub mod grid;

#[derive(Debug)]
pub struct Color([f64; 3]);

impl Color {
    pub fn from_u64(v: u64) -> Self {
        Color([
            ((v >> 16) & 255) as f64 / 255f64,
            ((v >> 8) & 255) as f64 / 255f64,
            (v & 255) as f64 / 255f64,
        ])
    }
}

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
        index: i32,
    },
    Busy(bool),
    Flush,

    // Grid Events
    GridResize {
        grid: i32,
        width: u32,
        height: u32,
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

#[derive(Debug)]
pub enum CursorShape {
    Block,
    Horizontal,
    Vertical,
}

impl Default for CursorShape {
    fn default() -> Self {
        Self::Block
    }
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

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
    HlAttrDefine(Vec<HighlightAttr>),
    GridLine(Vec<GridLine>),
    GridClear,
    GridDestroy,
    GridCursorGoto {
        grid: i32,
        row: u32,
        column: u32,
    },
    GridScroll {
        grid: i32,
        top: u32,
        bottom: u32,
        left: u32,
        right: u32,
        rows: u32,
        columns: u32,
    },
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

#[derive(Debug, Default)]
pub struct RgbAttr {
    pub foreground: Option<Color>,
    pub background: Option<Color>,
    pub special: Option<Color>,

    pub reverse: bool,
    pub italic: bool,
    pub bold: bool,
    pub underline: bool,
    pub undercurl: bool,
}

#[derive(Debug)]
pub struct HighlightAttr {
    pub id: i32,
    pub rgb_attr: RgbAttr,
    pub cterm_attr: RgbAttr,
}

#[derive(Debug)]
pub struct GridLine {
    pub grid: i32,
    pub row: u32,
    pub col_start: u32,
    pub cells: Vec<GridCell>,
}

#[derive(Debug)]
pub struct GridCell {
    pub text: String,
    pub hl_id: u64,
    pub repeated: u64,
}

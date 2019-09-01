use super::Color;

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

#[derive(Debug)]
pub struct GridGoto {
    pub grid: i32,
    pub row: u32,
    pub column: u32,
}

#[derive(Debug)]
pub struct GridScroll {
    pub grid: i32,
    pub top: u32,
    pub bottom: u32,
    pub left: u32,
    pub right: u32,
    pub rows: u32,
    pub columns: u32,
}

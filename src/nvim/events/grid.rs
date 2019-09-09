use super::Color;

#[derive(Debug, Default, Copy, Clone)]
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
    pub id: usize,
    pub rgb_attr: RgbAttr,
    pub cterm_attr: RgbAttr,
}

#[derive(Debug)]
pub struct GridLine {
    pub grid: usize,
    pub row: usize,
    pub col_start: usize,
    pub cells: Vec<GridCell>,
}

#[derive(Debug)]
pub struct GridCell {
    pub text: String,
    pub hl_id: usize,
    pub repeated: usize,
}

#[derive(Debug)]
pub struct GridGoto {
    pub grid: usize,
    pub row: usize,
    pub column: usize,
}

#[derive(Debug)]
pub struct GridScroll {
    pub grid: usize,
    pub top: usize,
    pub bottom: usize,
    pub left: usize,
    pub right: usize,
    pub rows: i64,
    pub columns: usize,
}

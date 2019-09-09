#[derive(Debug, Copy, Clone, Default)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
    pub shape: CursorShape,
}

impl Cursor {
    pub fn move_to(mut self, row: usize, col: usize) -> Self {
        self.row = row;
        self.col = col;

        self
    }

    pub fn change_shape(mut self, shape: CursorShape) -> Self {
        self.shape = shape;

        self
    }
}

#[derive(Debug, Copy, Clone)]
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

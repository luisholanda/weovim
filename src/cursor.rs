use crate::neovim::CursorShape;

#[derive(Debug, Default, Copy, Clone)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
    pub shape: CursorShape,
}

impl Cursor {
    pub fn move_to(&mut self, row: usize, col: usize) {
        self.row = row;
        self.col = col;
    }

    pub fn change_shape(&mut self, shape: CursorShape) {
        self.shape = shape;
    }
}

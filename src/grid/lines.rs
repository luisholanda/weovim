use super::*;
use crate::neovim::{GridCell, GridLine};

const NEOVIM_MAXCOMBINE: usize = 6;

/// A sequence of Unicode code points that represent a single grapheme.
/// Holds up to six (`maxcombine` in Neovim) UTF-8 encoded code points.
type GraphemeCluster = [char; NEOVIM_MAXCOMBINE];

const EMPTY_GRAPHEME: GraphemeCluster = [' '; 6];

#[derive(Debug, Copy, Clone)]
pub struct LineCell {
    chr: GraphemeCluster,
    size: usize,
    hl_id: u64,
}

impl Default for LineCell {
    fn default() -> Self {
        LineCell {
            chr: EMPTY_GRAPHEME,
            size: 0,
            hl_id: u64::max_value(),
        }
    }
}

impl LineCell {
    fn new(cell: GridCell<'_>) -> Self {
        let mut default = Self::default();
        default.update(cell);

        default
    }

    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }

    fn update(&mut self, cell: GridCell<'_>) {
        self.size = 0;

        for (idx, chr) in cell.text.chars().take(NEOVIM_MAXCOMBINE).enumerate() {
            self.chr[idx] = chr;
            self.size += 1;
        }

        self.hl_id = cell.hl_id;
    }

    fn render_in(self, sectioned: &mut SectionedLine<u64>) {
        for ch in &self.chr[..self.size] {
            sectioned.text.push(*ch);
        }
    }
}

pub(super) type Line = [LineCell];

pub(super) fn update(line: &mut Line, new_line: GridLine<'_>) {
    let mut cells = &mut line[new_line.col_start as usize..];

    for gc in new_line.cells {
        let (cells_to_update, remainers) = cells.split_at_mut(gc.repeated as usize);
        cells = remainers;

        let new_cell = LineCell::new(gc);
        cells_to_update.fill(new_cell);
    }
}

pub(super) fn render(line: &Line, sectioned: &mut SectionedLine<u64>) {
    if let Some((fc, cells)) = line.split_first() {
        fc.render_in(sectioned);

        let mut hl = fc.hl_id;
        let mut old_len = 0;

        for cell in cells {
            if cell.hl_id != hl && sectioned.text.len() != old_len {
                sectioned.sections.push(Section {
                    hl,
                    start: old_len,
                    end: sectioned.text.len(),
                });

                hl = cell.hl_id;
                old_len = sectioned.text.len();
            }

            cell.render_in(sectioned);
        }
    }
}

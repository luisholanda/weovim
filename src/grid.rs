pub use self::lines::LineCell;
use crate::editor::HighlightGroups;
use crate::neovim::GridLine;
use fnv::FnvHashSet;
use std::cmp::Ordering;

mod lines;
pub mod rendered;

use rendered::RenderedLines;

/// A grid line sectioned by a property `H` with underlying text `T`.
///
/// "Sectioned" here means that we found all the continuous slices of text
/// that have the same value for some property `H`. Normally `H` will be
/// the highlight group of the text.
#[derive(Debug, Default)]
pub struct SectionedLine<H, T = String> {
    /// The full line text.
    pub text: T,
    /// The sections of text with same value for `H`.
    pub sections: Vec<Section<H>>,
}

impl<H> SectionedLine<H> {
    fn clear(&mut self) {
        self.text.clear();
        self.sections.clear();
    }
}

impl<T: Clone, H: Clone> Clone for SectionedLine<H, T> {
    fn clone(&self) -> Self {
        Self {
            text: self.text.clone(),
            sections: self.sections.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.text.clone_from(&source.text);
        self.sections.clone_from(&source.sections);
    }
}

/// A slice of a grid line that have the same value for a property `H`.
#[derive(Debug, Copy, Clone)]
pub struct Section<H> {
    /// The value of the property.
    pub hl: H,
    /// The start of the slice.
    pub start: usize,
    /// The end of the slice, exclusive.
    pub end: usize,
}

/// A line grid.
///
/// Represents an entire Neovim grid.
#[derive(Debug, Default)]
pub struct Lines {
    /// Continuous cells of the grid.
    ///
    /// Each line is stored in the slice `cells[rows * cols..(rows+1) * cols]`.
    ///
    /// The `Vec` should always have size `rows * cols`.
    cells: Vec<lines::LineCell>,
    /// Number of rows (that is, lines) in the grid.
    rows: usize,
    /// Number of columns in each line of the grid.
    cols: usize,
    /// Cached rendering of lines.
    ///
    /// Allocations are re-used between renderings.
    cached_sections: Vec<SectionedLine<u64>>,
    /// Lines that were modified and should be re-renderized.
    dirty_lines: FnvHashSet<usize>,
}

impl Clone for Lines {
    fn clone(&self) -> Self {
        Self {
            cells: self.cells.clone(),
            rows: self.rows,
            cols: self.cols,
            cached_sections: self.cached_sections.clone(),
            dirty_lines: self.dirty_lines.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.cells.clone_from(&source.cells);
        self.rows = source.rows;
        self.cols = source.cols;
        self.cached_sections.clone_from(&source.cached_sections);
        self.dirty_lines.clone_from(&source.dirty_lines);
    }
}

impl Lines {
    /// Updates the grid with the received neovim update.
    pub fn update_line(&mut self, grid_line: GridLine) {
        self.dirty_line(grid_line.row as usize);
        lines::update(self.line_at_mut(grid_line.row as usize), grid_line)
    }

    /// Resize the grid to `rows x columns`.
    pub fn resize(&mut self, rows: usize, columns: usize) {
        self.dirty_all();
        self.cached_sections.resize_with(rows, || SectionedLine {
            text: String::with_capacity(columns),
            sections: Vec::with_capacity(columns / 2),
        });

        self.rows = rows;
        self.cols = columns;

        self.cells
            .resize_with(rows * columns, lines::LineCell::default);
    }

    /// Clears the entire grid.
    pub fn clear(&mut self) {
        self.dirty_all();

        for cell in &mut self.cells {
            cell.clear();
        }
    }

    /// Scroll a region of `grid`.
    ///
    /// This is semantically unrelated to editor |scrolling|, rather this is
    /// an optimized way to say "copy these screen cells".
    ///
    /// The following diagrams show what happens per scroll direction.
    ///
    /// * "===" represents the SR (scroll region) boundaries.
    /// * "---" represents the moved rectangles.
    ///
    /// Note that dst and src share a common region.
    ///
    /// If `rows` is bigger than 0, move a rectangle in the SR up, this can
    /// happen while scrolling down.
    ///
    ///         +-------------------------+
    ///         | (clipped above SR)      |            ^
    ///         |=========================| dst_top    |
    ///         | dst (still in SR)       |            |
    ///         +-------------------------+ src_top    |
    ///         | src (moved up) and dst  |            |
    ///         |-------------------------| dst_bot    |
    ///         | src (invalid)           |            |
    ///         +=========================+ src_bot
    ///
    /// If `rows` is less than zero, move a rectangle in the SR down, this can
    /// happen while scrolling up.
    ///
    ///         +=========================+ src_top
    ///         | src (invalid)           |            |
    ///         |------------------------ | dst_top    |
    ///         | src (moved down) and dst|            |
    ///         +-------------------------+ src_bot    |
    ///         | dst (still in SR)       |            |
    ///         |=========================| dst_bot    |
    ///         | (clipped below SR)      |            v
    ///         +-------------------------+
    ///
    /// `cols` is always zero in this version of Nvim, and reserved for future
    /// use.
    pub fn scroll(&mut self, reg: [usize; 4], rows: i64) {
        let range = match 0.cmp(&rows) {
            Ordering::Greater => Stride::Asc(reg[0], (reg[1] as i64 - rows + 1) as usize),
            Ordering::Less => Stride::Desc(reg[1], (reg[0] as i64 - rows - 1) as usize),
            // When `rows == 0`, we aren't scrolling anything, so we can just return.
            Ordering::Equal => return,
        };

        let left = reg[2];
        let line_range = reg[3] - left;

        // This is needed to guarantee the safety of the unsafe block.
        if self.cols < left {
            panic!("Called scroll with region bigger that line!");
        }

        for i in range {
            self.dirty_line(i);

            // SAFETY: As rows != 0, src and dst will guaranteed be different
            // lines and thus, non-overlapping regions (src_idx != i).
            unsafe {
                let src_idx = (i as i64 + rows) as usize;

                let dst = self.line_at_mut(i).as_mut_ptr().add(left);
                let src = self.line_at_mut(src_idx).as_mut_ptr().add(left);

                std::ptr::swap_nonoverlapping(src, dst, line_range);
            }
        }
    }

    /// Render the dirty grid lines.
    pub fn render(&mut self) {
        for row in &self.dirty_lines {
            let start = row * self.cols;
            let end = start + self.cols;

            let line = &mut self.cells[start..end];
            lines::render(line, &mut self.cached_sections[*row]);
        }
        self.dirty_lines.clear();
    }

    pub fn rendered_lines<'l>(&'l self, hl_groups: &'l HighlightGroups) -> RenderedLines<'l> {
        RenderedLines::new(&self.cached_sections, hl_groups)
    }

    fn line_at_mut(&mut self, row: usize) -> &mut lines::Line {
        let start = row * self.cols;
        let end = start + self.cols;

        &mut self.cells[start..end]
    }

    fn dirty_all(&mut self) {
        for i in 0..self.rows {
            self.dirty_lines.insert(i);
            self.cached_sections[i].clear();
        }
    }

    fn dirty_line(&mut self, i: usize) {
        if self.dirty_lines.insert(i) {
            self.cached_sections[i].clear();
        }
    }
}

enum Stride {
    Asc(usize, usize),
    Desc(usize, usize),
}

impl Iterator for Stride {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        match *self {
            Self::Asc(start, stop) if start < stop => {
                (*self) = Self::Asc(start + 1, stop);

                Some(start)
            }
            Self::Desc(start, stop) if start > stop => {
                (*self) = Self::Desc(start - 1, stop);

                Some(start)
            }
            _ => None,
        }
    }
}

use crate::editor::HighlightGroups;
use crate::nvim::events::grid::GridLine;
use fnv::FnvHashSet;

mod lines;
pub mod rendered;

use rendered::RenderedLines;

/// A grid line sectioned by a property `H` with underlying text `T`.
///
/// "Sectioned" here means that we already calculated all the sections in the line's
/// text, meaning that we found all the continuous slices of text that have the same
/// value for some property `H`. Normally `H` will be the highlight group of the text.
#[derive(Debug, Default)]
pub struct SectionedLine<H, T = String> {
    /// The full line text.
    pub text: T,
    /// The sections of text with same value for `H`.
    pub sections: Vec<Section<H>>,
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
    cached_sections: Vec<SectionedLine<usize>>,
    /// Lines that were modified and should be re-renderized.
    dirty_lines: FnvHashSet<usize>,
}

impl Lines {
    /// Updates the lines using a batch of modifications sent by Neovim.
    pub fn update_lines(&mut self, grid_lines: Vec<GridLine>) {
        for gl in grid_lines {
            self.dirty_line(gl.row);

            let line = self.line_at_mut(gl.row);
            lines::update(line, gl);
        }
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
    /// 	+-------------------------+
    /// 	| (clipped above SR)      |            ^
    /// 	|=========================| dst_top    |
    /// 	| dst (still in SR)       |            |
    /// 	+-------------------------+ src_top    |
    /// 	| src (moved up) and dst  |            |
    /// 	|-------------------------| dst_bot    |
    /// 	| src (invalid)           |            |
    /// 	+=========================+ src_bot
    ///
    /// If `rows` is less than zero, move a rectangle in the SR down, this can
    /// happen while scrolling up.
    ///
    /// 	+=========================+ src_top
    /// 	| src (invalid)           |            |
    /// 	|------------------------ | dst_top    |
    /// 	| src (moved down) and dst|            |
    /// 	+-------------------------+ src_bot    |
    /// 	| dst (still in SR)       |            |
    /// 	|=========================| dst_bot    |
    /// 	| (clipped below SR)      |            v
    /// 	+-------------------------+
    ///
    /// `cols` is always zero in this version of Nvim, and reserved for future
    /// use.
    pub fn scroll(&mut self, reg: [usize; 4], rows: i64) {
        let range = if rows > 0 {
            Stride::Asc(reg[0], (reg[1] as i64 - rows + 1) as usize)
        } else if rows < 0 {
            Stride::Desc(reg[1], (reg[0] as i64 - rows - 1) as usize)
        } else {
            // When `rows == 0`, we aren't scrolling anything, so we ca
            // just return.
            return;
        };

        let left = reg[2];
        let line_range = reg[3] - left;

        // This is needed to guarantee the safety of the unsafe block.
        if self.cols < left {
            panic!("Called scroll with region bigger that line!");
        }

        for i in range {
            self.dirty_line(i);

            // As rows != 0, src and dst will guaranteed be
            // non-overlapping regions (src_idx != i).
            let src_idx = (i as i64 + rows) as usize;

            unsafe {
                let dst = self.line_at_mut(i).as_mut_ptr().add(left);
                let src = self.line_at_mut(src_idx).as_mut_ptr().add(left);

                std::ptr::swap_nonoverlapping(src, dst, line_range);
            }
        }
    }

    /// Render the grid lines using a set of highlight groups.
    pub fn render<'l>(&'l mut self, hl_groups: &'l HighlightGroups) -> RenderedLines<'l> {
        for row in &self.dirty_lines {
            let start = row * self.cols;
            let end = start + self.cols;

            let line = &mut self.cells[start..end];
            lines::render(line, &mut self.cached_sections[*row]);
        }
        self.dirty_lines.clear();

        RenderedLines::new(&self.cached_sections, hl_groups)
    }

    #[inline]
    fn line_at_mut(&mut self, row: usize) -> &mut lines::Line {
        let start = row * self.cols;
        let end = start + self.cols;

        &mut self.cells[start..end]
    }

    #[inline]
    fn dirty_all(&mut self) {
        for i in 0..self.rows {
            self.dirty_line(i);
        }
    }

    fn dirty_line(&mut self, i: usize) {
        if self.dirty_lines.insert(i) {
            let cache = &mut self.cached_sections[i];

            cache.sections.clear();
            cache.text.clear();
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

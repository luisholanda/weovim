use crate::editor::HighlightGroups;
use crate::nvim::events::grid::{GridLine, RgbAttr};

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

pub type RenderedLines<'l> = Vec<SectionedLine<&'l RgbAttr, &'l str>>;

#[derive(Debug, Default)]
pub struct Lines {
    cells: Vec<Option<LineCell>>,
    cached_sections: Vec<Cache<SectionedLine<usize>>>,
    rows: usize,
    cols: usize,
}

impl Lines {
    pub fn update_lines(&mut self, grid_lines: Vec<GridLine>) {
        for gl in grid_lines {
            self.cached_sections[gl.row].invalidate();

            let line = self.line_at_mut(gl.row);
            lines::update(line, gl);
        }
    }

    pub fn resize(&mut self, rows: usize, columns: usize) {
        self.rows = rows;
        self.cols = columns;

        self.clear_cache();
        self.cached_sections.resize_with(rows, Cache::default);

        self.cells.resize(rows * columns, None);
    }

    pub fn clear(&mut self) {
        self.clear_cache();

        for cell in &mut self.cells {
            cell.take();
        }
    }

    pub fn scroll(&mut self, reg: [usize; 4], rows: i64) {
        let range = if rows > 0 {
            Stride::Asc(reg[0], (reg[1] as i64 - rows + 1) as usize)
        } else if rows < 0 {
            Stride::Desc(reg[1], (reg[0] as i64 - rows - 1) as usize)
        } else {
            // This return guarantees that the unsafe operation will
            // be safe.
            return;
        };

        let left = reg[2];
        let right = reg[3];

        for i in range {
            self.cached_sections[i].invalidate();

            unsafe {
                // As rows != 0, src and dst will guaranteed be
                // non-overlapping regions (src_idx != i).
                let src_idx = (i as i64 + rows) as usize;
                assert!(src_idx != i);

                let dst = self.line_at_mut(i).as_mut_ptr().add(left);
                let src = self.line_at_mut(src_idx).as_mut_ptr().add(left);

                // Swap src[left..=right] with dst[left..=right]
                std::ptr::swap_nonoverlapping(src, dst, right - left);
            }
        }
    }

    pub fn render<'l>(&'l mut self, hl_groups: &'l HighlightGroups) -> RenderedLines<'l> {
        use std::time::Instant;
        let now = Instant::now();

        for (row, cache) in self.cached_sections.iter_mut().enumerate() {
            if !cache.valid {
                let start = row * self.cols;
                let end = start + self.cols;

                let line = &mut self.cells[start..end];
                cache.update(|val| lines::render(line, val));
            }
        }

        let lines = self
            .cached_sections
            .iter()
            .map(|cl| SectionedLine {
                text: cl.value.text.as_str(),
                sections: cl
                    .value
                    .sections
                    .iter()
                    .map(|s| Section {
                        hl: hl_groups.group(s.hl),
                        start: s.start,
                        end: s.end,
                    })
                    .collect(),
            })
            .collect();

        println!("Render took {}ns", now.elapsed().as_nanos());

        lines
    }

    #[inline]
    fn clear_cache(&mut self) {
        for cs in &mut self.cached_sections {
            cs.invalidate();
        }
    }

    #[inline]
    fn line_at_mut(&mut self, row: usize) -> &mut lines::Line {
        let start = row * self.cols;
        let end = start + self.cols;

        &mut self.cells[start..end]
    }
}

#[derive(Debug, Default)]
struct Cache<T> {
    value: T,
    valid: bool,
}

impl<T> Cache<T> {
    #[inline]
    fn invalidate(&mut self) {
        self.valid = false;
    }

    fn update(&mut self, f: impl FnOnce(&mut T)) {
        f(&mut self.value);
        self.valid = true;
    }
}

#[derive(Debug, Clone)]
struct LineCell {
    chr: String,
    hl_id: usize,
}

mod lines {
    use super::*;

    pub(super) type Line = [Option<LineCell>];

    pub(super) fn update(line: &mut Line, new_line: GridLine) {
        let mut cells = &mut line[new_line.col_start..];

        for gc in new_line.cells {
            let new_cell = Some(LineCell {
                chr: gc.text,
                hl_id: gc.hl_id,
            });

            for i in 0..gc.repeated - 1 {
                cells[i].clone_from(&new_cell);
            }

            cells[gc.repeated - 1] = new_cell;
            cells = &mut cells[gc.repeated..];
        }
    }

    pub(super) fn render(line: &Line, sectioned: &mut SectionedLine<usize>) {
        sectioned.sections.clear();
        sectioned.text.clear();

        sectioned.sections.reserve(line.len());

        let mut empty = 0usize;
        for c in line {
            if c.is_none() {
                sectioned.text.push(' ');
                empty += 1;
            } else {
                break;
            }
        }

        // The entire line is empty.
        if empty == line.len() {
            return;
        }

        // The first is guaranteed to be set.
        if let Some((Some(fc), cells)) = line[empty..].split_first() {
            sectioned.text.push_str(&fc.chr);
            sectioned.text.reserve(cells.len());

            let mut hl = fc.hl_id;
            let mut start = 0;

            for (end, cell) in cells.iter().enumerate() {
                if let Some(c) = cell {
                    sectioned.text.push_str(&c.chr);

                    if c.hl_id != hl {
                        sectioned.sections.push(Section { hl, start, end });

                        start = end;

                        hl = c.hl_id;
                    }
                } else {
                    sectioned.text.push(' ');
                }
            }
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

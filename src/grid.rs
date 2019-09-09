use crate::editor::HighlightGroups;
use crate::nvim::events::grid::{GridLine, RgbAttr};
use rayon::prelude::*;

#[derive(Debug, Copy, Clone)]
pub struct Section<H> {
    pub hl: H,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug)]
pub struct SectionedLine<H, T = String> {
    pub text: T,
    pub sections: Vec<Section<H>>,
}

pub type RenderedLines<'l> = Vec<SectionedLine<&'l RgbAttr, &'l str>>;

#[derive(Debug, Default)]
pub struct Lines {
    lines: Vec<Line>,
    cached_sections: Vec<Option<SectionedLine<usize>>>,
}

impl Lines {
    pub fn update_lines(&mut self, grid_lines: Vec<GridLine>) {
        for line in grid_lines {
            self.cached_sections[line.row].take();

            self.lines[line.row].update(line);
        }
    }

    pub fn resize(&mut self, rows: usize, columns: usize) {
        self.clear_cache();
        self.cached_sections.resize_with(rows, Option::default);

        let old_len = self.lines.len();

        self.lines
            .resize_with(rows, || Line::with_capacity(columns));
        for line in &mut self.lines[..old_len] {
            line.resize(columns);
        }
    }

    pub fn clear(&mut self) {
        self.clear_cache();

        for line in &mut self.lines {
            line.clear();
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
            self.cached_sections[i].take();

            unsafe {
                // As rows != 0, src and dst will guaranteed be
                // non-overlapping regions (src_idx > i).
                let src_idx = (i as i64 + rows) as usize;
                assert!(src_idx > i);

                let src: *mut _ = &mut self.lines[src_idx].cells[left];
                let dst: *mut _ = &mut self.lines[i].cells[left];

                let src = src.add(left);
                let dst = dst.add(left);

                // Swap src[left..=right] with dst[left..=right]
                std::ptr::swap_nonoverlapping(src, dst, right - left + 1);
            }
        }
    }

    pub fn render<'l>(&'l mut self, hl_groups: &'l HighlightGroups) -> RenderedLines<'l> {
        for (i, line) in self.lines.iter().enumerate() {
            if self.cached_sections[i].is_none() {
                self.cached_sections[i] = Some(line.render());
            }
        }

        self.cached_sections
            .iter()
            .flatten()
            .map(|cl| SectionedLine {
                text: cl.text.as_str(),
                sections: cl
                    .sections
                    .iter()
                    .map(|s| Section {
                        hl: hl_groups.group(s.hl),
                        start: s.start,
                        end: s.end,
                    })
                    .collect(),
            })
            .collect()
    }

    fn clear_cache(&mut self) {
        for cs in &mut self.cached_sections {
            (*cs) = None;
        }
    }
}

#[derive(Debug, Clone)]
struct LineCell {
    chr: String,
    hl_id: usize,
}

#[derive(Debug, Clone)]
struct Line {
    cells: Vec<Option<LineCell>>,
}

impl Line {
    fn with_capacity(cap: usize) -> Self {
        Self {
            cells: vec![None; cap],
        }
    }

    fn clear(&mut self) {
        for cell in &mut self.cells {
            cell.take();
        }
    }

    fn resize(&mut self, cap: usize) {
        self.cells.resize_with(cap, Option::default);
    }

    fn update(&mut self, line: GridLine) {
        let cells = &mut self.cells[line.col_start..];
        let mut idx = 0;

        for gc in line.cells {
            let new_cell = Some(LineCell {
                chr: gc.text,
                hl_id: gc.hl_id,
            });

            for _ in 0..gc.repeated - 1 {
                cells[idx] = new_cell.clone();
                idx += 1;
            }

            cells[idx] = new_cell;
            idx += 1;
        }
    }

    fn render(&self) -> SectionedLine<usize> {
        let mut sections = Vec::with_capacity(self.cells.len());

        let cells = self.cells.as_slice();

        let empty: usize = cells.iter().take_while(|c| c.is_none()).count();
        let mut text = " ".repeat(empty);

        if empty < self.cells.len() {
            // The first is guaranteed to be set.
            if let Some((Some(fc), cells)) = cells[empty..].split_first() {
                text.reserve(cells.len());

                let mut hl = fc.hl_id;
                let mut start = 0;
                let mut end = 0;

                text.push_str(&fc.chr);

                for c in cells {
                    if let Some(c) = c {
                        text.push_str(&c.chr);

                        if c.hl_id == hl {
                            end += 1;
                        } else {
                            sections.push(Section { hl, start, end });

                            start = end + 1;
                            end = start;

                            hl = c.hl_id;
                        }
                    } else {
                        text.push(' ');
                    }
                }
            }
        }

        SectionedLine { text, sections }
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

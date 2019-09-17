use crate::editor::HighlightGroups;
use crate::nvim::events::grid::{GridLine, RgbAttr};

#[derive(Debug, Copy, Clone)]
pub struct Section<H> {
    pub hl: H,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Default)]
pub struct SectionedLine<H, T = String> {
    pub text: T,
    pub sections: Vec<Section<H>>,
}

pub type RenderedLines<'l> = Vec<SectionedLine<&'l RgbAttr, &'l str>>;

#[derive(Debug, Default)]
pub struct Lines {
    lines: Vec<Line>,
    cached_sections: Vec<Cache<SectionedLine<usize>>>,
}

impl Lines {
    pub fn update_lines(&mut self, grid_lines: Vec<GridLine>) {
        for line in grid_lines {
            self.cached_sections[line.row].invalidate();

            self.lines[line.row].update(line);
        }
    }

    pub fn resize(&mut self, rows: usize, columns: usize) {
        self.clear_cache();
        self.cached_sections.resize_with(rows, Cache::default);

        let old_len = self.lines.len();

        if old_len < rows {
            self.lines.resize(rows, Line::with_capacity(columns));
        }

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
            self.cached_sections[i].invalidate();

            unsafe {
                // As rows != 0, src and dst will guaranteed be
                // non-overlapping regions (src_idx != i).
                let src_idx = (i as i64 + rows) as usize;
                assert!(src_idx != i);

                let dst = self.lines[i].cells.as_mut_ptr().add(left);
                let src = self.lines[src_idx].cells.as_mut_ptr().add(left);

                // Swap src[left..=right] with dst[left..=right]
                std::ptr::swap_nonoverlapping(src, dst, right - left);
            }
        }
    }

    pub fn render<'l>(&'l mut self, hl_groups: &'l HighlightGroups) -> RenderedLines<'l> {
        for (line, cache) in self.lines.iter().zip(self.cached_sections.iter_mut()) {
            if !cache.valid {
                cache.update(|val| line.render(val));
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

        lines
    }

    fn clear_cache(&mut self) {
        for cs in &mut self.cached_sections {
            cs.invalidate();
        }
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
        let mut cells = &mut self.cells[line.col_start..];

        for gc in line.cells {
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

    fn render(&self, sectioned: &mut SectionedLine<usize>) {
        sectioned.sections.clear();
        sectioned.text.clear();

        sectioned.sections.reserve(self.cells.len());

        let mut empty = 0usize;
        for c in &self.cells {
            if c.is_none() {
                sectioned.text.push(' ');
                empty += 1;
            } else {
                break;
            }
        }

        if empty < self.cells.len() {
            // The first is guaranteed to be set.
            if let Some((Some(fc), cells)) = self.cells[empty..].split_first() {
                sectioned.text.push_str(&fc.chr);
                sectioned.text.reserve(cells.len());

                let mut hl = fc.hl_id;
                let mut start = 0;
                let mut end = 0;

                for c in cells {
                    if let Some(c) = c {
                        sectioned.text.push_str(&c.chr);

                        if c.hl_id == hl {
                            end += 1;
                        } else {
                            sectioned.sections.push(Section { hl, start, end });

                            start = end + 1;
                            end = start;

                            hl = c.hl_id;
                        }
                    } else {
                        sectioned.text.push(' ');
                    }
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

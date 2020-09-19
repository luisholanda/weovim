use super::*;

#[derive(Debug)]
pub(super) struct LineCell {
    chr: String,
    hl_id: usize,
}

impl Clone for LineCell {
    fn clone(&self) -> Self {
        LineCell {
            chr: self.chr.clone(),
            hl_id: self.hl_id,
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.chr.clone_from(&source.chr);
        self.hl_id = source.hl_id;
    }
}

impl Default for LineCell {
    fn default() -> Self {
        LineCell {
            chr: String::from(" "),
            hl_id: usize::max_value(),
        }
    }
}

impl LineCell {
    pub(super) fn clear(&mut self) {
        self.chr.clear();
        self.chr.push(' ');
        self.hl_id = usize::max_value();
    }
}

pub(super) type Line = [LineCell];

pub(super) fn update(line: &mut Line, new_line: GridLine) {
    let mut cells = &mut line[new_line.col_start..];

    for gc in new_line.cells {
        let new_cell = LineCell {
            chr: gc.text,
            hl_id: gc.hl_id,
        };

        cells
            .iter_mut()
            .take(gc.repeated)
            .skip(1)
            .for_each(|cell| cell.clone_from(&new_cell));

        cells[0] = new_cell;
        cells = &mut cells[gc.repeated..];
    }
}

pub(super) fn render(line: &Line, sectioned: &mut SectionedLine<usize>) {
    if let Some((fc, cells)) = line.split_first() {
        sectioned.text.push_str(&fc.chr);

        let mut hl = fc.hl_id;
        let mut old_len = 0;

        for cell in cells {
            if cell.hl_id != hl && sectioned.text.len() != old_len {
                log::trace!(target: "line-render", "New section: hl - {}, text - {}", hl, &sectioned.text[old_len..]);

                sectioned.sections.push(Section {
                    hl,
                    start: old_len,
                    end: sectioned.text.len(),
                });

                hl = cell.hl_id;
                old_len = sectioned.text.len();
            }

            sectioned.text.push_str(&cell.chr);
        }
    }
}

use super::{Section, SectionedLine};
use crate::editor::HighlightGroups;
use crate::nvim::events::grid::RgbAttr;

pub struct RenderedLines<'l> {
    lines: &'l [SectionedLine<usize>],
    hl_groups: &'l HighlightGroups,
    curr_line: usize,
}

impl<'l> RenderedLines<'l> {
    pub(super) fn new(lines: &'l [SectionedLine<usize>], hl_groups: &'l HighlightGroups) -> Self {
        Self {
            lines,
            hl_groups,
            curr_line: 0,
        }
    }
}

impl<'l> Iterator for RenderedLines<'l> {
    type Item = RenderedLine<'l>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr_line >= self.lines.len() {
            return None;
        }

        let line = &self.lines[self.curr_line];
        self.curr_line += 1;

        Some(RenderedLine::new(line, self.hl_groups))
    }
}

pub struct RenderedLine<'l> {
    hl_groups: &'l HighlightGroups,
    text: &'l str,
    sections: &'l [Section<usize>],
}

impl<'l> RenderedLine<'l> {
    fn new(sectioned: &'l SectionedLine<usize>, hl_groups: &'l HighlightGroups) -> Self {
        Self {
            text: &sectioned.text,
            sections: &sectioned.sections,
            hl_groups,
        }
    }

    pub fn text(&self) -> &'l str {
        self.text
    }

    pub fn n_sections(&self) -> usize {
        self.sections.len()
    }
}

use super::{Section, SectionedLine};
use crate::editor::HighlightGroups;
use crate::neovim::RgbAttr;

pub struct RenderedLines<'l> {
    lines: &'l [SectionedLine<u64>],
    hl_groups: &'l HighlightGroups,
}

impl<'l> RenderedLines<'l> {
    pub(super) fn new(lines: &'l [SectionedLine<u64>], hl_groups: &'l HighlightGroups) -> Self {
        Self { lines, hl_groups }
    }
}

impl<'l> Iterator for RenderedLines<'l> {
    type Item = RenderedLine<'l>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((next, rem)) = self.lines.split_first() {
            self.lines = rem;

            Some(RenderedLine::new(next, self.hl_groups))
        } else {
            None
        }
    }
}

pub struct RenderedLine<'l> {
    hl_groups: &'l HighlightGroups,
    text: &'l str,
    sections: &'l [Section<u64>],
}

impl<'l> RenderedLine<'l> {
    fn new(sectioned: &'l SectionedLine<u64>, hl_groups: &'l HighlightGroups) -> Self {
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

impl<'l> Iterator for RenderedLine<'l> {
    type Item = (&'l str, RgbAttr);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((next, rem)) = self.sections.split_first() {
            self.sections = rem;

            let content = &self.text[next.start..next.end];
            let rgb_attr = self.hl_groups.group_color_set(next.hl);

            Some((content, rgb_attr))
        } else {
            None
        }
    }
}

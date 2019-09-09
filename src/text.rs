use crate::grid::{Section, SectionedLine};
use crate::nvim::events::grid::RgbAttr;
use std::collections::HashMap;

pub struct Text<'e, SL> {
    lines: &'e mut SL,
    hl_groups: &'e HashMap<usize, RgbAttr>,
}

impl<'e, SL> Text<'e, SL>
where
    SL: Iterator<Item = SectionedLine>
{
    pub fn new(lines: &'e mut SL, hl_groups: &'e HashMap<usize, RgbAttr>) -> Self {
        Text {
            lines,
            hl_groups,
        }
    }
}

impl<'e, SL> Iterator for Text<'e, SL>
where
    SL: Iterator<Item = SectionedLine>
{
    type Item = (&str, )
}


pub struct TextLine {
}

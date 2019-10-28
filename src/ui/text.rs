use super::graphics::{gpu::Gpu, Point};
use super::Color;
use crate::grid::rendered::*;
use crate::nvim::events::grid::RgbAttr;
use std::f32;

#[derive(Debug)]
pub struct Text<'a> {
    /// The content of the text.
    pub content: &'a str,
    /// Size of the text.
    pub size: f32,
    /// Color of the text.
    pub color: Color,
    /// Background of the text.
    pub background: Color,
}

pub struct UiGridRender<'g, 'f> {
    lines: RenderedLines<'g>,
    curr_line: Option<RenderedLine<'g>>,
    position: Point,
    text_size: f32,
    line_height: f32,
    gpu: &'g mut Gpu<'f>,
}

impl<'g, 'f> UiGridRender<'g, 'f> {
    pub fn build(lines: RenderedLines<'g>) -> UiGridRenderBuilder<'g> {
        UiGridRenderBuilder {
            lines,
            position: Point::origin(),
            text_size: 0.0,
            line_height_multiplier: 0.0,
        }
    }

    fn render(mut self) {
        let mut position = self.position;

        for line in self.lines {
            let mut final_pos = position;

            for text in line {
                final_pos = self.gpu.queue_text(position, text);
                position.x = final_pos.x;
            }

            position.y = self.line_height * final_pos.y;
        }
    }
}

pub struct UiGridRenderBuilder<'g> {
    lines: RenderedLines<'g>,
    position: Point,
    text_size: f32,
    line_height_multiplier: f32,
}

impl<'g> UiGridRenderBuilder<'g> {
    pub fn start_from(&mut self, position: Point) -> &mut Self {
        self.position = position;

        self
    }

    pub fn with_text_size(&mut self, size: f32) -> &mut Self {
        self.text_size = size;

        self
    }

    pub fn with_line_height_multiplier(&mut self, multiplier: f32) -> &mut Self {
        self.line_height_multiplier = multiplier;

        self
    }

    pub fn render<'f>(self, gpu: &'g mut Gpu<'f>) {
        UiGridRender {
            lines: self.lines,
            curr_line: None,
            position: self.position,
            text_size: self.text_size,
            line_height: self.line_height_multiplier,
            gpu,
        }
        .render()
    }
}

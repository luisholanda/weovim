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

    fn render(self) {
        let mut position = self.position;
        let mut last_height = 0.0f32;

        for line in self.lines {
            let mut sections = Vec::with_capacity(line.n_sections());

            for mut text in line {
                text.size = self.text_size;
                sections.push(text);
            }

            if let Some((min, max)) = self.gpu.font.add_line(position, &sections) {
                log::debug!(target: "grid-render", "Rendered line bounded in {} and {}", min, max);

                last_height = self.line_height * (max.y - min.y);
            }

            position.y += last_height;
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
    pub fn start_from(mut self, position: Point) -> Self {
        self.position = position;

        self
    }

    pub fn with_text_size(mut self, size: f32) -> Self {
        self.text_size = size;

        self
    }

    pub fn with_line_height_multiplier(mut self, multiplier: f32) -> Self {
        self.line_height_multiplier = multiplier;

        self
    }

    pub fn render<'f>(self, gpu: &'g mut Gpu<'f>) {
        UiGridRender {
            lines: self.lines,
            position: self.position,
            text_size: self.text_size,
            line_height: self.line_height_multiplier,
            gpu,
        }
        .render()
    }
}

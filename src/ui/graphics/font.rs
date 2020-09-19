use super::transform::Transformation;
use super::Point;
use crate::ui::text::Text;
use wgpu_glyph::{
    BuiltInLineBreaker, GlyphBrush, GlyphCruncher, Layout, Scale, Section, SectionText,
    VariedSection,
};

pub struct Font<'f> {
    brusher: GlyphBrush<'f, ()>,
    layout: Layout<BuiltInLineBreaker>,
    space_width: f32,
    space_height: f32,
}

impl<'f> Font<'f> {
    /// Creates a new [Font] using the raw bytes of the desired font.
    pub fn from_bytes(device: &mut wgpu::Device, font_bytes: &'f [u8]) -> Self {
        let mut instance = Self {
            brusher: wgpu_glyph::GlyphBrushBuilder::using_font_bytes(font_bytes)
                .texture_filter_method(wgpu::FilterMode::Nearest)
                .initial_cache_size((512, 512))
                .build(device, wgpu::TextureFormat::Bgra8UnormSrgb),
            layout: Layout::default_single_line()
                .h_align(wgpu_glyph::HorizontalAlign::Left)
                .v_align(wgpu_glyph::VerticalAlign::Top),
            space_width: 0.0,
            space_height: 0.0,
        };

        let font = instance.brusher.fonts().first().unwrap();

        let space_glyph = font.glyph('_').standalone();
        let space_data = space_glyph.get_data().unwrap();
        let extents = space_data.extents.unwrap();

        instance
    }

    /// Enqueue a [Text] to be drawn.
    ///
    /// Returns the `(min, max)` bounds of the text.
    pub fn add(&mut self, position: Point, text: &Text) -> (Point, Point) {
        log::debug!(target: "font", "Enqueueing render of text '{}' at {}", text.content, position);

        let start_displ = self.start_space_displacement(text.content);

        let section = VariedSection {
            screen_position: (position.x + start_displ, position.y),
            text: vec![SectionText {
                text: text.content,
                scale: Scale::uniform(text.size),
                color: text.color.into_raw_components(),
                font_id: wgpu_glyph::FontId::default(),
            }],
            layout: self.layout,
            ..VariedSection::default()
        };

        let bounds = self.brusher.glyph_bounds(&section);

        log::trace!(target: "font", "Enqueued text bounds: {:?}", bounds);

        self.brusher.queue(section);

        if let Some(bounds) = bounds {
            let start = Point::new(bounds.min.x, bounds.min.y);
            let end = Point::new(bounds.max.x + self.end_space_displacement(text.content), bounds.max.y);

            (start, end)
        } else {
            let max_x = position.x + start_displ;
            let max_y = position.y + self.space_height;

            (position, Point::new(max_x, max_y))
        }
    }

    pub fn add_line(&mut self, position: Point, line: &[Text]) -> Option<(Point, Point)> {
        log::debug!(target: "font", "Enqueueing line with {} sections at {}", line.len(), position);

        let mut sections = Vec::with_capacity(line.len());

        for text in line {
            sections.push(SectionText {
                text: &text.content,
                scale: Scale::uniform(text.size),
                color: text.color.into_raw_components(),
                font_id: wgpu_glyph::FontId::default(),
            });
        }

        let mut section = VariedSection {
            screen_position: (position.x, position.y),
            text: sections,
            layout: self.layout,
            ..VariedSection::default()
        };

        let bounds = self.brusher.glyph_bounds(&section);

        self.brusher.queue(section);

        bounds.map(|b| (Point::new(b.min.x, b.min.y), Point::new(b.max.x, b.max.y)))
    }

    /// Draw the enqueued texts into the target texture.
    pub fn draw(
        &mut self,
        device: &mut wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        width: u32,
        height: u32,
    ) {
        self.brusher
            .draw_queued(device, encoder, target, width, height)
            .expect("Failed to draw text");
    }

    fn start_space_displacement(&self, text: &str) -> f32 {
        let mut displacement = 0.0f32;

        for chr in text.chars() {
            if chr.is_whitespace() {
                displacement += self.space_width;
            } else {
                break;
            }
        }

        displacement
    }

    fn end_space_displacement(&self, text: &str) -> f32 {
        let mut displacement = 0.0f32;

        for chr in text.chars().rev() {
            if chr.is_whitespace() {
                displacement += self.space_width;
            } else {
                break;
            }
        }

        displacement
    }
}

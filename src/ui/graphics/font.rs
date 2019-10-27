use super::transform::Transformation;
use crate::ui::rect::Rect;
use crate::ui::text::Text;
use std::borrow::Cow;
use wgpu_glyph::{GlyphBrush, GlyphCruncher, Scale, Section, SectionText, VariedSection};

pub struct Font<'f> {
    glyphs: GlyphBrush<'f, ()>,
}

impl<'f> Font<'f> {
    /// Creates a new [Font] using the raw bytes of the desired font.
    pub fn from_bytes(device: &mut wgpu::Device, font_bytes: &'f [u8]) -> Self {
        Self {
            glyphs: wgpu_glyph::GlyphBrushBuilder::using_font_bytes(font_bytes)
                .texture_filter_method(wgpu::FilterMode::Nearest)
                .build(device, wgpu::TextureFormat::Bgra8UnormSrgb),
        }
    }

    /// Enqueue a [Text] to be drawn.
    pub fn add<'t>(&mut self, text: &Text<'t>) {
        let section: Section<'t> = text.into();
        self.glyphs.queue(section);
    }

    /// Enqueue many [Text]s to be drawn.
    pub fn add_many<'t>(&mut self, texts: impl Iterator<Item = Text<'t>>, total: Option<usize>) {
        let mut sections = Vec::with_capacity(total.unwrap_or_default());

        for text in texts {
            sections.push(SectionText {
                text: &text.content,
                scale: wgpu_glyph::Scale {
                    x: text.position.x,
                    y: text.position.y,
                },
                color: text.color.into_raw_components(),
                font_id: wgpu_glyph::FontId::default(),
            });
        }

        let section = VariedSection {
            text: sections,
            layout: wgpu_glyph::Layout::default_single_line()
                .h_align(wgpu_glyph::HorizontalAlign::Center)
                .v_align(wgpu_glyph::VerticalAlign::Center),
            ..Default::default()
        };

        self.glyphs.queue(section)
    }

    // Measures the bounds of a [Text].
    //
    // This can be used to find the [Rect] that can be used to render
    // the background of the recived text.
    pub fn bounds_of<'t>(&mut self, text: &Text<'t>) -> Option<Rect> {
        let section: Section<'t> = text.into();

        self.glyphs.glyph_bounds(section)
    }

    /// Draw the enqueued texts into the target texture.
    pub fn draw(
        &mut self,
        device: &mut wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        transform: Transformation,
    ) {
        self.glyphs
            .draw_queued_with_transform(device, encoder, target, transform.into())
            .expect("Failed to draw text");
    }
}

impl<'t, 'a> From<&'t Text<'a>> for Section<'a> {
    fn from(text: &'t Text<'a>) -> Self {
        Self {
            text: &text.content,
            screen_position: (text.position.x, text.position.y),
            scale: wgpu_glyph::Scale {
                x: text.size,
                y: text.size,
            },
            color: text.color.into_raw_components(),
            layout: wgpu_glyph::Layout::default()
                .h_align(wgpu_glyph::HorizontalAlign::Center)
                .v_align(wgpu_glyph::VerticalAlign::Center),
            ..Default::default()
        }
    }
}

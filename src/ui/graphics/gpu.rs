use winit::window::Window;

use super::color::Color;
use super::font::Font;
use super::quad::{Pipeline, Quad};
use super::transform::Transformation;
use super::Point;
use crate::ui::Text;

pub struct Gpu<'f> {
    surface: wgpu::Surface,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    quad: Pipeline,
    pub font: Font<'f>,
}

pub struct Target {
    pub width: u16,
    pub height: u16,
    transformation: Transformation,
    swap_chain: wgpu::SwapChain,
}

impl<'f> Gpu<'f> {
    pub(in crate::ui) fn for_window(window: &Window, font_bytes: &'f [u8]) -> Option<Self> {
        let adapter = wgpu::Adapter::request(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            backends: wgpu::BackendBit::all(),
        })?;

        let (mut device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            extensions: wgpu::Extensions {
                anisotropic_filtering: false,
            },
            limits: wgpu::Limits::default(),
        });

        let surface = wgpu::Surface::create(window);
        let font = Font::from_bytes(&mut device, font_bytes);
        let quad = Pipeline::new(&mut device);

        Some(Self {
            surface,
            adapter,
            device,
            queue,
            font,
            quad,
        })
    }

    pub(in crate::ui) fn target(&self, width: u16, height: u16) -> Target {
        let swap_chain = self.device.create_swap_chain(
            &self.surface,
            &wgpu::SwapChainDescriptor {
                usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                width: width as u32,
                height: height as u32,
                present_mode: wgpu::PresentMode::Vsync,
            },
        );

        Target {
            width,
            height,
            transformation: Transformation::identity(),
            swap_chain,
        }
    }

    pub(in crate::ui) fn queue_text(&mut self, position: Point, text: Text) -> (Point, Point) {
        let (min, max) = self.font.add(position, &text);

        self.quad.enqueue(Quad {
            position: [min.x, min.y],
            scale: [max.x - min.x, max.y - min.y],
            color: text.background.into_raw_components(),
            border_radius: 0.0,
        });

        (min, max)
    }

    pub(in crate::ui) fn draw(&mut self, target: &mut Target, background_color: Color) {
        let frame = target.swap_chain.get_next_texture();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

        {
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    load_op: wgpu::LoadOp::Clear,
                    store_op: wgpu::StoreOp::Store,
                    clear_color: background_color.into(),
                }],
                depth_stencil_attachment: None,
            });
        }

        self.quad.draw(
            &mut self.device,
            &mut encoder,
            &frame.view,
            target.transformation,
        );

        self.font.draw(
            &mut self.device,
            &mut encoder,
            &frame.view,
            target.width as u32,
            target.height as u32
        );

        self.queue.submit(&[encoder.finish()]);
    }
}

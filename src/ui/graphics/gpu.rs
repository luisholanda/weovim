use winit::window::Window;

use super::color::Color;
use super::font::Font;
use super::quad::Pipeline;
use super::transform::Transformation;

pub struct Gpu<'f> {
    surface: wgpu::Surface,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    font: Font<'f>,
    quad: Pipeline,
}

pub struct Target {
    width: u16,
    height: u16,
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
                present_mode: wgpu::PresentMode::NoVsync,
            },
        );

        Target {
            width,
            height,
            transformation: Transformation::orthographic(width as f32, height as f32),
            swap_chain,
        }
    }

    pub(in crate::ui) fn draw(&mut self, target: &mut Target, background_color: Color) {
        let frame = target.swap_chain.get_next_texture();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

        let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &frame.view,
                resolve_target: None,
                load_op: wgpu::LoadOp::Clear,
                store_op: wgpu::StoreOp::Store,
                clear_color: background_color.into(),
            }],
            depth_stencil_attachment: None,
        });

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
            target.transformation,
        );

        self.queue.submit(&[encoder.finish()]);
    }
}

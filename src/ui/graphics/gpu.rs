use winit::window::Window;

use super::font::Font;
use super::transform::Transformation;

pub struct Gpu<'f> {
    surface: wgpu::Surface,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    font: Font<'f>,
}

pub struct Target {
    width: u16,
    height: u16,
    transformation: Transformation,
    swap_chain: wgpu::SwapChain
}

impl<'f> Gpu<'f> {
    pub(in crate::ui) fn for_window(
        window: &Window,
        font_bytes: &'f [u8],
    ) -> Option<Self> {
        let adapter = wgpu::Adapter::request(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            backends: wgpu::BackendBit::all()
        })?;

        let (mut device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            extensions: wgpu::Extensions {
                anisotropic_filtering: false,
            },
            limits: wgpu::Limits::default(),
        });

        let surface = wgpu::Surface::create(window);
        let font = Font::from_bytes(&mut device, font_bytes);

        Some(Self {
            surface,
            adapter,
            device,
            queue,
            font,
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
                present_mode: wgpu::PresentMode::NoVsync
            },
        );

        Target {
            width,
            height,
            transformation: Transformation::orthographic(width as f32, height as f32),
            swap_chain,
        }
    }
}

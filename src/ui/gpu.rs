use crate::color::Color;
use winit::dpi::PhysicalSize;

pub struct Gpu {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    swap_chain_descr: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    size: PhysicalSize<u32>,
}

impl Gpu {
    pub(super) async fn new(window: &super::UiWindow) -> Self {
        let size = window.size();

        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window.raw()) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    shader_validation: true,
                },
                None,
            )
            .await
            .unwrap();

        let swap_chain_descr = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let swap_chain = device.create_swap_chain(&surface, &swap_chain_descr);

        Self {
            surface,
            device,
            queue,
            swap_chain_descr,
            swap_chain,
            size,
        }
    }

    pub fn color_format(&self) -> wgpu::TextureFormat {
        self.swap_chain_descr.format
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.size = new_size;
        self.swap_chain_descr.width = new_size.width;
        self.swap_chain_descr.height = new_size.height;
        self.swap_chain = self
            .device
            .create_swap_chain(&self.surface, &self.swap_chain_descr);
    }

    pub fn create_shader_module(&self, source: wgpu::ShaderModuleSource) -> wgpu::ShaderModule {
        self.device.create_shader_module(source)
    }

    pub fn create_pipeline_layout(
        &self,
        layout_desc: &wgpu::PipelineLayoutDescriptor,
    ) -> wgpu::PipelineLayout {
        self.device.create_pipeline_layout(&layout_desc)
    }

    pub fn create_render_pipeline(
        &self,
        descr: &wgpu::RenderPipelineDescriptor,
    ) -> wgpu::RenderPipeline {
        self.device.create_render_pipeline(descr)
    }

    pub fn begin_render(&mut self, bg: Color) -> (wgpu::SwapChainTexture, wgpu::CommandEncoder) {
        let frame = self
            .swap_chain
            .get_current_frame()
            .expect("Timeout getting current frame")
            .output;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("weovim::ui::gpu command encoder"),
            });

        // Clear the frame surface.
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &frame.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(bg.into()),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });

        (frame, encoder)
    }

    pub fn finish_render(&mut self, frame: wgpu::SwapChainTexture, encoder: wgpu::CommandEncoder) {
        self.queue.submit(std::iter::once(encoder.finish()));
    }
}

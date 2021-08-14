use crate::color::Color;
use crate::ui::renderers::Quad;
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

const APPROX_GRID_SIZE: u64 = 72 * 160;
const APPROX_QUAD_SIZE: u64 = 16;
const STAGING_BELT_SIZE: wgpu::BufferAddress =
    APPROX_GRID_SIZE * std::mem::size_of::<Quad>() as u64 / APPROX_QUAD_SIZE;

pub struct Gpu {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    swap_chain_descr: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    size: PhysicalSize<u32>,
    staging_belt: wgpu::util::StagingBelt,
}

impl Gpu {
    pub(super) async fn new(window: &super::UiWindow) -> Self {
        let size = window.size();

        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        // winit Window is a valid window handle.
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

        let staging_belt = wgpu::util::StagingBelt::new(STAGING_BELT_SIZE);

        Self {
            surface,
            device,
            queue,
            swap_chain_descr,
            swap_chain,
            size,
            staging_belt,
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

    pub fn create_buffer(&self, descr: &wgpu::BufferDescriptor<'_>) -> wgpu::Buffer {
        self.device.create_buffer(descr)
    }

    pub fn create_buffer_init(&self, descr: &wgpu::util::BufferInitDescriptor<'_>) -> wgpu::Buffer {
        self.device.create_buffer_init(descr)
    }

    pub fn write_buffer(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::Buffer,
        offset: wgpu::BufferAddress,
        size: wgpu::BufferSize,
    ) -> wgpu::BufferViewMut {
        self.staging_belt
            .write_buffer(encoder, target, offset, size, &self.device)
    }

    pub fn begin_render(&mut self) -> (wgpu::SwapChainTexture, wgpu::CommandEncoder) {
        let frame = self
            .swap_chain
            .get_current_frame()
            .expect("Timeout getting current frame")
            .output;

        let encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("weovim::ui::gpu command encoder"),
            });

        (frame, encoder)
    }

    pub fn clear(
        &mut self,
        frame: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        bg: Color,
    ) {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: frame,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(bg.into()),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });
    }

    pub fn finish_render(&mut self, frame: wgpu::SwapChainTexture, encoder: wgpu::CommandEncoder) {
        self.staging_belt.finish();

        self.queue.submit(std::iter::once(encoder.finish()));

        tokio::spawn(self.staging_belt.recall());
    }
}

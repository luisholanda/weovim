use crate::color::Color;
use crate::editor::{UiStateFromEditor, TripleBufferReader, UiEditorEvent};
use crate::neovim::Neovim;
use std::sync::Arc;
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Mutex,
};
use winit::dpi::*;
use winit::event::*;
use winit::event_loop::{ControlFlow, EventLoop};
#[cfg(target_os = "macos")]
use winit::platform::macos::WindowBuilderExtMacOS;
use winit::window::{CursorIcon, Window, WindowBuilder};

mod gpu;
mod renderers;
mod shaper;
use self::renderers::Quad;

pub struct Ui {
    gpu: Mutex<gpu::Gpu>,
    quad: Mutex<renderers::QuadRenderer>,
    neovim: Mutex<Neovim>,
    window: UiWindow,
    input: UiInput,
}

impl Ui {
    pub async fn new(neovim: Neovim) -> (Arc<Ui>, UiEventLoop) {
        let event_loop = <EventLoop<UiEditorEvent>>::with_user_event();
        let window = UiWindow::build(&event_loop);

        let mut gpu = gpu::Gpu::new(&window).await;
        let quad = renderers::QuadRenderer::new(&mut gpu);

        let ui = Arc::new(Ui {
            gpu: Mutex::new(gpu),
            quad: Mutex::new(quad),
            neovim: Mutex::new(neovim),
            window,
            input: Default::default(),
        });

        let ui_event_loop = UiEventLoop {
            ui: ui.clone(),
            event_loop,
        };

        (ui, ui_event_loop)
    }

    fn render(&self) {
        let mut gpu = self.gpu.lock().unwrap();
        let (frame, mut encoder) = gpu.begin_render();
        gpu.clear(&frame.view, &mut encoder, Color::BLACK);

        {
            let mut quad = self.quad.lock().unwrap();

            quad.queue(Quad::new(0.25, 0.25, 0.5, 0.5, Color::WHITE));
            quad.render_in(&frame.view, &mut encoder, &mut gpu);
        }

        gpu.finish_render(frame, encoder);
    }
}

struct UiInput {
    modifiers_state: ModifiersState,
    mouse_hold: Option<MouseButton>,
    is_typing: bool,
}

impl Default for UiInput {
    fn default() -> Self {
        Self {
            modifiers_state: ModifiersState::empty(),
            mouse_hold: None,
            is_typing: false,
        }
    }
}

impl UiInput {
    fn is_holding(&self) -> bool {
        self.mouse_hold.is_some()
    }
}

struct UiWindow {
    winit_window: Window,
    window_status: WindowStatus,
}

/// Window manipulation methods.
impl UiWindow {
    fn build(event_loop: &EventLoop<UiEditorEvent>) -> Self {
        let builder = WindowBuilder::new()
            .with_resizable(true)
            .with_visible(false)
            .with_transparent(true)
            .with_title("WeoVim");

        // Hide the decorations
        #[cfg(target_os = "macos")]
        let builder = builder
            .with_title_hidden(true)
            .with_titlebar_buttons_hidden(true)
            .with_titlebar_transparent(true)
            .with_fullsize_content_view(true);

        #[cfg(not(target_os = "macos"))]
        let builder = builder.with_decorations(false);

        let winit_window = builder.build(&event_loop).unwrap();

        // Don't let the window extend past the window monitor.
        let monitor = winit_window.current_monitor();
        winit_window.set_max_inner_size(monitor.map(|m| m.size()));

        Self {
            winit_window,
            window_status: WindowStatus::Suspended,
        }
    }

    fn show_window(&self) {
        self.winit_window.set_visible(true);
    }

    fn hide_window(&self) {
        self.winit_window.set_visible(false);
    }

    fn request_redraw(&self) {
        self.winit_window.request_redraw()
    }

    fn set_title(&self, title: &str) {
        self.winit_window.set_title(title)
    }

    fn show_cursor(&self) {
        self.winit_window.set_cursor_visible(true)
    }

    fn hide_cursor(&self) {
        self.winit_window.set_cursor_visible(false)
    }

    fn set_cursor_icon(&self, icon: CursorIcon) {
        self.winit_window.set_cursor_icon(icon)
    }

    fn change_window_status(&mut self, new_status: WindowStatus) -> bool {
        std::mem::replace(&mut self.window_status, new_status) != new_status
    }
}

/// Size-related methods.
impl UiWindow {
    fn scale_factor(&self) -> f64 {
        self.winit_window.scale_factor()
    }

    fn size(&self) -> PhysicalSize<u32> {
        self.winit_window.inner_size()
    }

    fn logical_size(&self) -> LogicalSize<u32> {
        self.size().to_logical(self.scale_factor())
    }

    fn convert_to_logical<P: Pixel>(&self, position: PhysicalPosition<P>) -> LogicalPosition<u32> {
        position.to_logical(self.scale_factor())
    }

    fn convert_to_physical<P: Pixel>(&self, position: LogicalPosition<P>) -> PhysicalPosition<u32> {
        position.to_physical(self.scale_factor())
    }
}

impl UiWindow {
    pub(self) fn raw(&self) -> &Window {
        &self.winit_window
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
enum WindowStatus {
    Focused,
    Unfocused,
    Suspended,
}

impl WindowStatus {
    const fn is_low_power(self) -> bool {
        false
    }
}

pub struct UiEventLoop {
    ui: Arc<Ui>,
    event_loop: EventLoop<UiEditorEvent>,
}

impl UiEventLoop {
    pub fn run(self) -> ! {
        let ui = self.ui;
        ui.window.show_window();
        self.event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::MainEventsCleared => ui.window.request_redraw(),
                Event::RedrawRequested(_) => ui.render(),
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::Resized(new_physical_size) => {
                        // TODO: calculate margins to adjust window size to grid size.
                        // TODO: Discover new grid size and send to neovim.
                    }
                    WindowEvent::CloseRequested => {
                        // TODO: Send quit command to neovim.
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::DroppedFile(dropped_file_path) => {
                        // TODO: What to do when a file is dropped in the client?
                    }
                    WindowEvent::Focused(true) => {
                        // TODO: The window gained focus, stop FPS limiter
                    }
                    WindowEvent::Focused(false) => {
                        // TODO: The window lost focus, start FPS limiter
                    }
                    WindowEvent::KeyboardInput {
                        input,
                        is_synthetic,
                        ..
                    } if !is_synthetic => {
                        // TODO: Handle user input.
                    }
                    WindowEvent::ReceivedCharacter(ch) => {
                        // TODO: Handle unicode char input.
                    }
                    WindowEvent::ModifiersChanged(new_modifiers_state) => {
                        // TODO: Handle modifiers change
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        // TODO: Handle mouse movement.
                    }
                    WindowEvent::CursorEntered { .. } => {
                        // TODO: Handle cursor enter
                    }
                    WindowEvent::CursorLeft { .. } => {
                        // TODO: Handle cursor left
                    }
                    WindowEvent::MouseWheel { .. } => {
                        // TODO: Handle mouse scroll.
                    }
                    WindowEvent::MouseInput { .. } => {
                        // TODO: Handle mouse input
                    }
                    WindowEvent::ScaleFactorChanged { .. } => {
                        // TODO: Handle dpi changes
                    }
                    _ => {}
                },
                Event::Resumed => {
                    // TODO: Handle application resume
                }
                Event::Suspended => {
                    // TODO: Handle application suspension
                }
                _ => {}
            }
        });
    }
}

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::ops::Deref;
use crossbeam_channel::Receiver;
use winit::dpi;
use winit::event::{
    ElementState, Event, KeyboardInput, MouseButton, MouseScrollDelta, WindowEvent,
};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};


pub(super) struct UIWindow {
    window: Window,
    exit_flag: Arc<AtomicBool>
}

impl UIWindow {
    pub(super) fn run<F>(func: F) -> !
    where
        F: FnOnce(UIWindow, Receiver<UIEvent>) + Send + 'static
    {
        UIWindowBuilder::new().build(func)
    }

    pub(super) fn physical_size(&self) -> dpi::PhysicalSize {
        self.logical_size().to_physical(self.window.hidpi_factor())
    }

    #[inline(always)]
    pub(super) fn logical_size(&self) -> dpi::LogicalSize {
        self.window.inner_size()
    }

    pub fn exit(&self) {
        self.exit_flag.store(true, Ordering::SeqCst);
    }
}

impl Deref for UIWindow {
    type Target = Window;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

impl Drop for UIWindow {
    fn drop(&mut self) {
        self.exit_flag.store(true, Ordering::Release);
    }
}

pub(super) struct UIWindowBuilder {
    event_loop: EventLoop<()>,
}

impl UIWindowBuilder {
    fn new() -> Self {
        Self {
            event_loop: EventLoop::new(),
        }
    }

    pub(super) fn build<F>(self, func: F) -> !
    where
        F: FnOnce(UIWindow, Receiver<UIEvent>) + Send + 'static
    {
        let exit_flag: Arc<AtomicBool> = Arc::default();

        let window = UIWindow {
            window: WindowBuilder::new()
                .with_maximized(true)
                .build(&self.event_loop)
                .expect("Couldn't open a window!"),
            exit_flag: exit_flag.clone(),
        };

        let (tx, rx) = crossbeam_channel::bounded(1024);

        window.request_redraw();

        log::debug!("Spawning listener thread...");
        std::thread::spawn(move || {
            log::info!("Listener thread init");
            func(window, rx);
        });

        self.event_loop.run(move |event, _, control_flow| {
            macro_rules! send {
                ($ev: expr) => {{
                    let __event = $ev;
                    log::debug!(target: "window", "Sending UIEvent: {:?}", __event);
                    if tx.send(__event).is_err() {
                        *control_flow = ControlFlow::Exit;
                        return;
                    }
                }}
            };

            log::trace!(target: "window", "Received window event\n {:?}", event);

            if exit_flag.load(Ordering::Relaxed) {
                *control_flow = ControlFlow::Exit;
                return;
            }

            match event {
                Event::Resumed => send!(UIEvent::Resumed),
                Event::Suspended => send!(UIEvent::Suspended),
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::KeyboardInput { input, .. } => {
                        send!(UIEvent::KeyboardInput(input));
                    }
                    WindowEvent::Resized(size) => send!(UIEvent::Resized(size)),
                    WindowEvent::CloseRequested => send!(UIEvent::CloseRequested),
                    WindowEvent::ReceivedCharacter(chr) => send!(UIEvent::ReceivedChar(chr)),
                    WindowEvent::Focused(bl) => send!(UIEvent::Focused(bl)),
                    WindowEvent::CursorMoved { position, .. } => {
                        send!(UIEvent::CursorMoved(position.x as f32, position.y as f32));
                    }
                    WindowEvent::CursorLeft { .. } => send!(UIEvent::Suspended),
                    WindowEvent::CursorEntered { .. } => send!(UIEvent::Resumed),
                    WindowEvent::MouseWheel { delta, .. } => {
                        let ui_event = match delta {
                            MouseScrollDelta::LineDelta(_, y) => UIEvent::ScrollLines(y),
                            MouseScrollDelta::PixelDelta(pos) => UIEvent::Scroll(pos.y as f32),
                        };

                        send!(ui_event);
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        let ui_event = match state {
                            ElementState::Pressed => UIEvent::MousePressed(button),
                            ElementState::Released => UIEvent::MouseReleased(button),
                        };

                        send!(ui_event);
                    }
                    _ => {}
                },
                _ => {}
            }

            *control_flow = ControlFlow::Wait;
        })
    }
}

#[derive(Debug)]
pub(super) enum UIEvent {
    Resized(dpi::LogicalSize),
    CloseRequested,
    Scroll(f32),
    ScrollLines(f32),
    MousePressed(MouseButton),
    MouseReleased(MouseButton),
    CursorMoved(f32, f32),
    KeyboardInput(KeyboardInput),
    Focused(bool),
    ReceivedChar(char),
    Suspended,
    Resumed,
}

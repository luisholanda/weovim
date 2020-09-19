use crate::grid::rendered::*;
use crate::editor::{Editor, EventRes};
use crate::nvim::events::RedrawEvent;
use crossbeam_channel::{Receiver, select};
use neovim_lib::{Neovim, NeovimApi};

pub(self) mod graphics;
pub(self) mod rect;
pub(self) mod text;
pub(self) mod window;

pub use graphics::{Color, Point};
pub use text::Text;

use text::UiGridRender;
use window::UIEvent;
use std::ops::Deref;

pub fn start_ui(nvim: Neovim, nvim_events: Receiver<RedrawEvent>, editor: Editor) -> ! {
    window::UIWindow::run(move |window, ui_events| {
        let mut nvim = nvim;
        let mut editor = editor;

        let font = load_font(String::from("DelugiaCode NF"));

        let mut gpu = graphics::gpu::Gpu::for_window(&window, &font).expect("Failed to access GPU");
        let (width, height): (u32, u32) = window.physical_size().into();
        let mut target = {

            gpu.target(width as u16, height as u16)
        };

        nvim.ui_try_resize(width as i64, height as i64).expect("Failed to resize neovim");

        let mut grid_element_height = 12.0f32;
        let line_height_multiplier = 1.04;

        loop {
            select! {
                recv(ui_events) -> event => {
                    let event = match event {
                        Err(_) => continue,
                        Ok(event) => event,
                    };

                    match event {
                        UIEvent::CloseRequested => window.exit(),
                        UIEvent::Resized(_) => {
                            let (width, height): (u32, u32) = window.physical_size().into();
                            let width = width as u16;
                            let height = height as u16;

                            if width != target.width || height != target.height {
                                target = gpu.target(width, height);
                                nvim.ui_try_resize(width as i64, height as i64).expect("Failed to resize neovim");
                            }
                        }
                        _ => {}
                    }
                },
                recv(nvim_events) -> event => {
                    let event = match event {
                        Err(_) => continue,
                        Ok(event) => event,
                    };

                    match editor.handle_nvim_redraw_event(event) {
                        EventRes::Render => {
                            UiGridRender::build(editor.render())
                                .with_text_size(grid_element_height)
                                .with_line_height_multiplier(line_height_multiplier)
                                .render(&mut gpu);

                            gpu.draw(&mut target, Color::WHITE);
                        }
                        EventRes::Destroy => return,
                        EventRes::Resize(_, height) => {
                            let win_height = window.physical_size().height;

                            grid_element_height = (win_height / height as f64).round() as f32;
                        },
                        EventRes::NextOne => {}
                    }
                },
            }
        }
    });
}

fn load_font(name: String) -> impl Deref<Target = Vec<u8>> {
    use font_kit::source::SystemSource;
    use font_kit::family_name::FamilyName;
    use font_kit::properties::Properties;

    let font = SystemSource::new()
        .select_best_match(&[FamilyName::Title(name)], &Properties::new())
        .unwrap()
        .load()
        .unwrap();

    font.copy_font_data().unwrap()
}

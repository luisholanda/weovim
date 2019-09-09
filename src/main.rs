use neovim_lib::NeovimApi;

mod cursor;
mod editor;
mod grid;
mod nvim;
mod ui;

use editor::{Editor, EventRes};

fn main() {
    let (mut neovim, rx) = nvim::start_neovim();
    let mut edt = Editor::new();

    println!("{:?}", neovim.ui_try_resize(800, 600));

    for event in rx {
        match edt.handle_nvim_redraw_event(event) {
            EventRes::Render => edt.render(),
            EventRes::Destroy => return,
            EventRes::NextOne => {}
        }
    }
}

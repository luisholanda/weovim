mod cursor;
mod editor;
mod escaped;
mod grid;
mod nvim;
mod ui;

use editor::{Editor, EventRes};

fn main() {
    let (_, rx) = nvim::start_neovim();
    let mut edt = Editor::new();

    for event in rx {
        match edt.handle_nvim_redraw_event(event) {
            EventRes::Render => edt.render(),
            EventRes::Destroy => return,
            EventRes::NextOne => {}
        }
    }
}

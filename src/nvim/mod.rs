use crossbeam_channel::{Receiver, Sender};
use neovim_lib::{Handler, Neovim, RequestHandler, Session, UiAttachOptions, Value};

pub mod events;

pub fn start_neovim() -> (Neovim, Receiver<events::RedrawEvent>) {
    let mut session = Session::new_unix_socket("/tmp/nvim-socket").unwrap();

    let (tx, rx) = crossbeam_channel::unbounded();
    session.start_event_loop_handler(NeovimHandler { tx });

    let mut ui_options = UiAttachOptions::new();
    ui_options.set_rgb(true);
    ui_options.set_linegrid_external(true);

    let mut nvim = Neovim::new(session);

    nvim.ui_attach(80, 30, &ui_options)
        .expect("Failed to attach UI");

    (nvim, rx)
}

struct NeovimHandler {
    // Sender-side of the UI events channel.
    tx: Sender<events::RedrawEvent>,
}

impl RequestHandler for NeovimHandler {}

impl Handler for NeovimHandler {
    fn handle_notify(&mut self, name: &str, args: Vec<Value>) {
        if name == "redraw" {
            for arg in args {
                if let Value::Array(a) = arg {
                    if let Some(event) = events::parse::redrawcmd(a) {
                        if let Err(err) = self.tx.send(event) {
                            println!("Client disconnected!");
                        }
                    }
                }
            }
        }
    }
}

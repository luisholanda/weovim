use neovim_lib::{Handler, Neovim, RequestHandler, Session, UiAttachOptions, Value};

mod events;

pub fn start_neovim() -> Neovim {
    let mut session = Session::new_unix_socket("/tmp/nvim-socket").unwrap();
    session.start_event_loop_handler(NeovimHandler {});

    let mut ui_options = UiAttachOptions::new();
    ui_options.set_rgb(true);
    ui_options.set_linegrid_external(true);

    let mut nvim = Neovim::new(session);

    nvim.ui_attach(80, 30, &ui_options)
        .expect("Failed to attach UI");

    nvim
}

struct NeovimHandler {
    // Sender-side of the UI events channel.
//tx: Sender<()>,
// Pending events to be send over the UI channel.
//pending_events: Vec<()>,
}

impl RequestHandler for NeovimHandler {
    fn handle_request(&mut self, _name: &str, _args: Vec<Value>) -> Result<Value, Value> {
        Err("UI doesn't support requests".into())
    }
}

impl Handler for NeovimHandler {
    fn handle_notify(&mut self, name: &str, args: Vec<Value>) {
        if name == "redraw" {
            for arg in args {
                if let Value::Array(a) = arg {
                    let event = events::parse::redrawcmd(a);
                    println!("{:?}", event);
                }
            }
        }
    }
}

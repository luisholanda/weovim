#[doc(inline)]
pub use self::api::UiOptions;
#[doc(inline)]
pub use self::events::*;
#[doc(inline)]
pub use self::rpc::{EventListener, LoggerEventListener};
use self::rpc::{EventReceiver, RpcProcess};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};

mod api;
mod events;
pub(self) mod msg;
mod rpc;

/// A Neovim session instance.
///
/// Provides methods for the RPC API and to fetch current session state.
///
/// # Buzy
///
/// When the neovim process is busy (available with [`Neovim::is_busy`]), no RPC methods
/// wil be sent to it. This is done as the methods can stack and cause the process to be
/// more busy and the state to quickly change.
pub struct Neovim {
    rpc: RpcProcess,
    title: Arc<RwLock<String>>,
    buzy: Arc<AtomicBool>,
}

impl Neovim {
    /// Start a new session.
    ///
    /// The returned event receiver should be spawned in a executor, as to permit the application
    /// to receive the redraw events.
    pub fn start<L: EventListener>(listener: L) -> std::io::Result<(Self, NeovimEventLoop<L>)> {
        let (rpc, recv) = RpcProcess::spawn()?;

        let title = <Arc<RwLock<_>>>::default();
        let buzy = <Arc<AtomicBool>>::default();

        let listener = NeovimEventListener {
            title: title.clone(),
            buzy: buzy.clone(),
            listener,
        };

        let neovim = Self { rpc, title, buzy };
        let recv = NeovimEventLoop {
            receiver: recv,
            listener,
        };

        Ok((neovim, recv))
    }

    /// The title for the window as suggested by the neovim instance.
    pub fn title<'a>(&'a self) -> impl std::ops::Deref<Target = String> + 'a {
        self.title.read().unwrap()
    }

    /// Is the neovim instance buzy?
    pub fn is_busy(&self) -> bool {
        self.buzy.load(Ordering::SeqCst)
    }
}

pub struct NeovimEventLoop<L: EventListener> {
    receiver: EventReceiver,
    listener: NeovimEventListener<L>,
}

impl<L: EventListener> NeovimEventLoop<L> {
    pub async fn run_loop(self) -> std::io::Result<!> {
        self.receiver.start_loop(self.listener).await
    }
}

struct NeovimEventListener<L: EventListener> {
    title: Arc<RwLock<String>>,
    buzy: Arc<AtomicBool>,
    listener: L,
}

impl<L: EventListener> EventListener for NeovimEventListener<L> {
    fn on_redraw_event<'e>(&mut self, event: RedrawEvent<'e>) {
        match event {
            RedrawEvent::Busy(buzy) => {
                self.buzy.store(buzy, Ordering::SeqCst);
                self.listener.on_redraw_event(event)
            }
            RedrawEvent::SetTitle(title) => {
                {
                    let mut write = self.title.write().unwrap();
                    write.replace_range(.., title);
                }

                self.listener.on_redraw_event(RedrawEvent::SetTitle(title))
            }
            event => self.listener.on_redraw_event(event),
        }
    }
}

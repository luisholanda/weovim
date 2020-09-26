#![feature(const_fn, never_type, slice_fill, str_split_once)]

use self::editor::Editor;
use self::neovim::UiOptions;
use mimalloc::MiMalloc;

mod color;
mod cursor;
mod editor;
mod grid;
mod neovim;
mod ui;

#[global_allocator]
static GLOBAL_ALLOCATOR: MiMalloc = MiMalloc;

#[tokio::main(core_threads = 2, max_threads = 8)]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let (editor, mut ui_state) = Editor::new();
    let (mut neovim, recv) = neovim::Neovim::start(editor)?;

    tokio::spawn(async move {
        if let Err(error) = recv.run_loop().await {
            log::error!("Error in neovim event loop: {}", error);
            std::process::exit(1);
        }
    });

    tokio::spawn(async move {
        while let Some(ev) = ui_state.recv.recv().await {
            log::info!("Received UiEditorEvent: {:?}", ev)
        }
    });

    let (_, event_loop) = ui::Ui::new().await;

    neovim
        .ui_attach(80, 30, UiOptions::RGB | UiOptions::EXT_LINEGRID)
        .await?;
    log::info!("UI attached");

    event_loop.run();
}

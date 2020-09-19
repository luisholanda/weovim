#![feature(const_fn, never_type)]

use self::neovim::UiOptions;
use mimalloc::MiMalloc;

mod color;
mod cursor;
mod editor;
mod grid;
mod neovim;
//mod nvim;
//mod ui;

use editor::Editor;

#[global_allocator]
static GLOBAL_ALLOCATOR: MiMalloc = MiMalloc;

#[tokio::main(core_threads = 2, max_threads = 8)]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let (mut neovim, recv) = neovim::Neovim::start(neovim::LoggerEventListener)?;
    neovim
        .ui_attach(80, 30, UiOptions::RGB | UiOptions::EXT_LINEGRID)
        .await?;
    log::info!("UI attached");

    if let Err(error) = recv.run_loop().await {
        log::error!("Error in neovim event loop: {}", error);
        std::process::exit(1);
    } else {
        Ok(())
    }
}

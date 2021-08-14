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

fn start_runtime() -> std::io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .max_threads(8)
        .enable_io()
        .build()
}

fn main() -> std::io::Result<()> {
    env_logger::init();

    let runtime = start_runtime()?;
    let _rt_guard = runtime.enter();

    let (editor, mut ui_state) = Editor::new();
    let (mut neovim, recv) = neovim::Neovim::start(editor)?;

    runtime.spawn(async move {
        while let Some(ev) = ui_state.recv.recv().await {
            log::info!("Received UiEditorEvent: {:?}", ev)
        }
    });

    let (_, event_loop) = runtime.block_on(ui::Ui::new());

    runtime.spawn(async move {
        neovim
            .ui_attach(80, 30, UiOptions::RGB | UiOptions::EXT_LINEGRID)
            .await
            .expect("failed to attach to UI");
        log::info!("UI attached");

        if let Err(error) = recv.run_loop().await {
            log::error!("Error in neovim event loop: {}", error);
            std::process::exit(1);
        }
    });

    event_loop.run()
}

use super::Neovim;
use std::io;

bitflags::bitflags! {
    pub struct UiOptions: u16 {
        const RGB            = 0b0000_0000_0000_0001;
        const OVERRIDE       = 0b0000_0000_0000_0010;
        const EXT_CMDLINE    = 0b0000_0000_0000_0100;
        const EXT_HLSTATE    = 0b0000_0000_0000_1000;
        const EXT_LINEGRID   = 0b0000_0000_0001_0000;
        const EXT_MESSAGES   = 0b0000_0000_0010_0000;
        const EXT_MULTIGRID  = 0b0000_0000_0100_0000;
        const EXT_POPUPMENU  = 0b0000_0000_1000_0000;
        const EXT_TABLINE    = 0b0000_0001_0000_0000;
        const EXT_TERMCOLORS = 0b0000_0010_0000_0000;
    }
}

const UI_OPTION_TO_KEY_MAP: &[(&str, UiOptions)] = &[
    ("rgb", UiOptions::RGB),
    ("override", UiOptions::OVERRIDE),
    ("ext_cmdline", UiOptions::EXT_CMDLINE),
    ("ext_hlstate", UiOptions::EXT_HLSTATE),
    ("ext_linegrid", UiOptions::EXT_LINEGRID),
    ("ext_messages", UiOptions::EXT_MESSAGES),
    ("ext_multigrid", UiOptions::EXT_MULTIGRID),
    ("ext_popupmenu", UiOptions::EXT_POPUPMENU),
    ("ext_tabline", UiOptions::EXT_TABLINE),
    ("ext_termcolors", UiOptions::EXT_TERMCOLORS),
];

// UI RPC methods
impl Neovim {
    /// Activates UI events on the channel.
    ///
    /// Entry point of all UI clients. Allow `--embed` to continue startup. Implies that
    /// the client is ready to show the UI. Adds the client to the list of UIs.
    ///
    /// ### Note:
    ///
    /// > If multiple UI clients are attached, the global screen dimensions degrade to
    /// the smallest client. E.g. if client A requests 80x40 but client B requests 200x100
    /// the global screen has size 80x40.
    ///
    /// ### Parameters:
    ///
    /// - `width`: Requested screen columns
    /// - `height`: Requested screen rows
    /// - `options`: `UiOptions` to use for this client.
    pub async fn ui_attach(&mut self, width: u64, height: u64, opts: UiOptions) -> io::Result<()> {
        log::debug!(
            "nvim_ui_attach width={}, height={}, opts={:?}",
            width,
            height,
            opts
        );
        let mut rpc = self.rpc.rpc_method_forget("nvim_ui_attach", 3);

        rpc.add_u64_arg(width);
        rpc.add_u64_arg(height);

        rpc.start_map_arg(UI_OPTION_TO_KEY_MAP.len() as u32);

        for (key, opt) in UI_OPTION_TO_KEY_MAP {
            rpc.add_bool_pair(key, opts.contains(*opt));
        }

        rpc.send().await?;

        Ok(())
    }

    pub async fn ui_try_resize(&mut self, width: u64, height: u64) -> io::Result<()> {
        let mut rpc = self.rpc.rpc_method_forget("nvim_ui_try_resize", 2);

        rpc.add_u64_arg(width);
        rpc.add_u64_arg(height);

        rpc.send().await
    }

    /// Tell Neovim to resize a gird. Triggers a `grid_resize` event with the requested
    /// grid size or the maximum size if it exceeds size limits.
    ///
    /// On invalid grid handle, fails with error.
    ///
    /// ### Parameters:
    ///
    /// - `grid`: The handle of the grid to be changed.
    /// - `width`: The new requested screen columns.
    /// - `height`: The new requested screen rows.
    pub async fn ui_try_resize_grid(
        &mut self,
        grid: u64,
        width: u64,
        height: u64,
    ) -> io::Result<()> {
        let mut rpc = self.rpc.rpc_method_forget("nvim_try_resize_grid", 3);

        rpc.add_u64_arg(grid);
        rpc.add_u64_arg(width);
        rpc.add_u64_arg(height);

        rpc.send().await
    }
}

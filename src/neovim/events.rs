use super::msg;
use crate::color::Color;
use bumpalo::{collections::Vec, Bump};
use rmp::Marker;
use std::io;

/// Possible UI redraw events sent by neovim.
///
/// Events must be handled in-order. Nvim sends a "flush" event when it has
/// completed a redraw of the entire screen (so all windows have a consistent view
/// of buffer state, options, etc.). Multiple "redraw" batches may be sent before
/// the entire screen has been redrawn, with "flush" following only the last
/// batch. The user should only see the final state (when "flush" is sent), not
/// any intermediate state while processing part of the batch array, nor after
/// a batch not ending with "flush".
#[derive(Debug)]
pub enum RedrawEvent<'a> {
    // Global Events
    /// Set the window title.
    SetTitle(&'a str),
    /// Set the icon (minimized) window title.
    SetIcon(&'a str),
    /// Set the properties for received modes.
    ModeInfoSet {
        /// Indicates if the UI should set the cursor style.
        cursor_style_enabled: bool,
        /// List of received modes' properties.
        mode_infos: Vec<'a, ModeInfo>,
    },
    /// UI-related option changes.
    ///
    /// Triggered when the UI first connects to Nvim, and whenever an option
    /// is changed by the user or a plugin.
    ///
    /// Options are not represented here if their effects are communicated in
    /// other UI events. For example, instead of forwarding the `mouse` option
    /// value, the `mouse_on`, `mouse_off` UI events directly indicate if mouse
    /// support is active. Some options like `ambiwidth`, have already taken
    /// effect on the grid, where appropriate empty cells are added, however a
    /// UI might still use such options when rendering raw text sent from Nvim,
    /// like for `ui-cmdline`.
    OptionSet(UiOption<'a>),
    /// Editor mode changed.
    ///
    /// The payload is an index into the array emitted in the `ModeInfoSet`
    /// event. UI should change the cursor style according to the properties
    /// specified in the corresponding item. The set of modes reported will
    /// change in new versions of Nvim, for instance more submodes and temporary
    /// states might be represented as separate modes.
    ModeChange(u64),
    /// `mouse` was enabled/disabled in the current editor mode. Useful for
    /// a terminal UI, or other situations where Nvim mouse would conflict with
    /// other usages of the mouse. UIs may ignore this and always send mouse
    /// input, because `mouse` decides the behavior of `nvim_input` implicitly.
    Mouse(bool),
    /// Nvim started or stopped being busy, and possibly not responsive to
    /// user input. This could be indicated to the user by hiding the cursor.
    Busy(bool),
    /// Nvim is done redrawing the screen.
    ///
    /// For an implementation that renders to an internal buffer, this is the
    /// time to display the redrawn parts to the user.
    Flush,

    // Grid Events
    /// Resize a grid.
    ///
    /// If the grid wasn't seen by the client before, a new grid is being created
    /// with this size.
    GridResize {
        /// The grid being resized.
        grid: u64,
        /// The new numbers of columns of the grid.
        width: u64,
        /// The new numbers of rows of the grid.
        height: u64,
    },
    /// Sets the default colors to be used when none is provided by the
    /// highlight group.
    ///
    /// See [`DefaultColorSet`] for more info.
    DefaultColorsSet(DefaultColorSet),
    /// Add a new highlight group to the highlight table.
    ///
    /// See [`HighlightAttr`] for more info.
    HlAttrDefine(HighlightAttr),
    /// The builtin highlight group `name` was set to use the attributes `hl_id`
    /// defined by a previous `hl_attr_define` call.
    ///
    /// This event is not needed to render the grids which use attribute ids directly,
    /// but is useful for an UI which want to render its own elements with consistent
    /// highlighting. For instance an UI using `ui-popupmenu` events might use the
    /// `hl-Pmenu` family of builtin highlights.
    HlGroupSet {
        /// Name of the highlight group.
        name: &'a str,
        /// Id of the group.
        hl_id: u64,
    },
    /// Redraw a continuous part of a `row` on a `grid`.
    GridLine(GridLine<'a>),
    /// Clear a grid.
    GridClear(u64),
    /// The grid will not be used anymore and the UI can free any data associated
    /// with it.
    GridDestroy(u64),
    /// Moves the cursor position, and set the grid to the current one.
    ///
    /// This event willl be sent at most once in a `redraw` batch and indicates the
    /// visible cursor positio.
    GridCursorGoto(GridGoto),
    /// Scroll a region of grid.
    ///
    /// This is semantically unrelated to editor _scrolling_, rather this an optimized
    /// way to say "copy these screen cells".
    ///
    /// The scrolled-in area will be filled using `GridLine` directly after the scroll
    /// event. The UI thus doesn't need to clear this area as part of handling the scroll
    /// event.
    GridScroll(GridScroll),

    // Multigrid Events
    /// Set the position and size of the grid in Nvim (i.e. the outer grid size).
    ///
    /// If the window was previously hidden, it should now be shown again.
    WinPos(WinPos),
    /// Display or reconfigure a floating window.
    WinFloatPos(WinFloatPos),
    /// Display or reconfigure an external window.
    ///
    /// The window should be displayed as a separate top-level window in the desktop
    /// environment or something similar.
    WinExternalPos {
        /// The grid to be shown in the window.
        grid: u64,
        /// The window that should be shown.
        win: WinNr,
    },
    /// Stop displaying the window.
    WinHide(WinNr),
    /// Close the window.
    WinClose(WinNr),
    /// Display messages on a grid.
    ///
    /// When `UiOptions::MESSAGES` is active, no message grid is used, and this event
    /// will not be sent.
    MsgSetPos(MsgSetPos<'a>),
    /// Indicates the range of buffer text displayed in the window, as well as the
    /// cursor position in the buffer. All positions are zero-based.
    WinViewPort(WinViewPort),
}

/// Properties of a mode.
#[derive(Debug)]
pub struct ModeInfo {
    /// The shape of the cursor to be used when the editor is this mode.
    pub cursor_shape: CursorShape,
    /// The percentage of the cell used by the cursor.
    pub cell_percentage: f64,
    /// Cursor highlight group id.
    ///
    /// When this is 0, the background and foreground colors should be
    /// swapped.
    pub attr_id: u64,
}

/// Possible shapes of the cursor.
#[derive(Debug, Copy, Clone)]
pub enum CursorShape {
    Block,
    Horizontal,
    Vertical,
}

/// An option related to the UI.
#[derive(Debug)]
pub enum UiOption<'a> {
    /// An option with a string value.
    ///
    /// Possible options:
    ///
    /// * `ambiwidth`
    /// * `guifontwide`
    /// * `guifont`
    /// * `guifontset`
    String { option: &'a str, value: &'a str },
    /// An option with an integer value.
    ///
    /// Possible options:
    ///
    /// * `pumblend`
    /// * `showtabline`
    /// * `ttimeoutlen`
    /// * `linespace`
    Int { option: &'a str, value: i64 },
    /// An option with a boolean value.
    ///
    /// Possible options:
    ///
    /// * `arabicshape`
    /// * `termguicolors`
    /// * `emoji`
    /// * `mousefocus`
    /// * `ttimeout`
    /// * `ext_linegrid`
    /// * `ext_multigrid`
    /// * `ext_hlstate`
    /// * `ext_termcolors`
    /// * `ext_cmdline`
    /// * `ext_popupmenu`
    /// * `ext_tabline`
    /// * `ext_wildmenu`
    /// * `ext_messages`
    Bool { option: &'a str, value: bool },
}

/// Default colors to be used when none is available in the section
/// highlight group.
///
/// **NOTE:** Unlike the corresponding `ui-grid-old` events, the screen
/// is not always cleared after sending this event. The UI must repaint
/// the screen with changed background color itself.
#[derive(Debug, Copy, Clone)]
pub struct DefaultColorSet {
    /// The default foreground color.
    pub foreground: Color,
    /// The default background color.
    pub background: Color,
    /// The default special color.
    pub special: Color,
}

/// Defines a new highlight group with the given id.
///
/// Id 0 will always be used for the default highlight with colors defined
/// by [`DefaultColorsSet`] and no styles applied.
///
/// **NOTE:** Nvim may reuse `id` value if tis internal highlight table is
/// full. In that case Nvim will always issue redraws of screen cells that
/// are affected by redefined ids, so UIs do not need to keep track of this
/// themselves.
#[derive(Debug)]
pub struct HighlightAttr {
    /// Index of the highlight group.
    pub id: u64,
    /// RGB properties of the group.
    pub rgb_attr: RgbAttr,
}

/// RGB properties of a highlight group.
///
/// For absent color keys the default color should be used. Don't store
/// the default value in the table, rather a sentinel value, so that a
/// changed default color will take effect.
#[derive(Debug, Default, Copy, Clone)]
pub struct RgbAttr {
    /// Foreground color.
    pub foreground: Option<Color>,
    /// Background color.
    pub background: Option<Color>,
    /// Color to use for underline and undercurl, when present.
    pub special: Option<Color>,
    /// Blend level (0-100).
    ///
    /// Could be used by UIs to support blending floating windows to the
    /// background or to signal a transparent cursor.
    pub blend: u8,
    flags: RgbAttrFlags,
}

impl RgbAttr {
    /// Reverse video. Foreground and background colors are switched.
    pub const fn reverse(&self) -> bool {
        self.flags.contains(RgbAttrFlags::REVERSE)
    }

    /// Italic text.
    pub const fn italic(&self) -> bool {
        self.flags.contains(RgbAttrFlags::ITALIC)
    }

    /// Bold text.
    pub const fn bold(&self) -> bool {
        self.flags.contains(RgbAttrFlags::BOLD)
    }

    /// Strikethrough text.
    pub const fn strikethrough(&self) -> bool {
        self.flags.contains(RgbAttrFlags::STRIKETHROUGH)
    }

    /// Underlined text. The line has `special` color.
    pub const fn underline(&self) -> bool {
        self.flags.contains(RgbAttrFlags::UNDERLINE)
    }

    /// Undercurl text. The line has `special` color.
    pub const fn undercurl(&self) -> bool {
        self.flags.contains(RgbAttrFlags::UNDERCURL)
    }
}

bitflags::bitflags! {
    #[derive(Default)]
    pub(super) struct RgbAttrFlags: u8 {
        const REVERSE       = 0b0000_0001;
        const ITALIC        = 0b0000_0010;
        const BOLD          = 0b0000_0100;
        const STRIKETHROUGH = 0b0000_1000;
        const UNDERLINE     = 0b0001_0000;
        const UNDERCURL     = 0b0010_0000;
    }
}

/// Redraws a continuous part of `row` on a `grid`, starting at the column
/// `col_start`.
///
/// If `hl_id` is not present the most recently seen `hl_id` in the same call
/// should be used (it is always sent for the first cell in the event).
///
/// The right cell of a double-width char will be represented as the empty
/// string.
///
/// If the array of cell changes doesn't reach to the end of the line, the
/// rest should remain unchanged. A whitespace char, repeated enough to cover
/// the remaining line, willl be sent when the rest of the line should be cleared.
#[derive(Debug)]
pub struct GridLine<'a> {
    /// The grid being redrawn.
    pub grid: u64,
    /// The row being redrawn.
    pub row: u64,
    /// From which column the redrawn should be made.
    pub col_start: u64,
    /// Cells being written in the grid.
    pub cells: Vec<'a, GridCell<'a>>,
}

/// Content of a grid cell.
#[derive(Debug)]
pub struct GridCell<'a> {
    /// Text that should be put in a cell.
    ///
    /// This will be an individual grapheme cluster.
    pub text: &'a str,
    /// The highlight group to be used in the cell.
    pub hl_id: u64,
    /// Number of times that this cell should be repeated in the row.
    pub repeated: u64,
}

/// Makes `grid` the current grid and `row`, `column` the cursor position
/// on this grid.
#[derive(Default, Debug)]
pub struct GridGoto {
    /// The current grid.
    pub grid: u64,
    /// The row position of the cursor.
    pub row: u64,
    /// The column position of the cursor.
    pub column: u64,
}

/// The grid region to be scrolled.
#[derive(Default, Debug)]
pub struct GridScroll {
    /// The target grid.
    pub grid: u64,
    /// The top row of the region.
    pub top: u64,
    /// The bottom row of the region.
    pub bottom: u64,
    /// The left column of the region.
    pub left: u64,
    /// The right column of the region.
    pub right: u64,
    /// How many rows should be scrolled.
    ///
    /// If positive, the region should be scrolled up, otherwise, it should
    /// be scrolled down.
    pub rows: i64,
}

/// Position and size of a window.
#[derive(Default, Debug, Copy, Clone)]
pub struct WinPos {
    /// The grid to be drawn in the window.
    pub grid: u64,
    /// The window's number.
    pub win: WinNr,
    /// The start row of the grid in the window.
    pub start_row: u64,
    /// The start column of the grid in the window.
    pub start_col: u64,
    /// The number of columns of the window.
    pub width: u64,
    /// The number of rows of the window.
    pub height: u64,
}

/// Position of a floating window in relation to another grid.
#[derive(Default, Debug, Copy, Clone)]
pub struct WinFloatPos {
    /// The grid to be drawn in the floating window.
    pub grid: u64,
    /// The window's number.
    pub win: WinNr,
    /// How the window should be anchored in relation to the anchor grid.
    pub anchor: WinFloatAnchor,
    /// The grid to which the floating window should be anchored.
    pub anchor_grid: u64,
    /// The anchor row of the anchor grid.
    pub anchor_row: u64,
    /// The anchor column of the anchor grid.
    pub anchor_col: u64,
    /// Can the floating window be focused?
    pub focusable: bool,
}

/// Anchor modes of a floating window.
#[derive(Debug, Copy, Clone)]
pub enum WinFloatAnchor {
    Northwest,
    Northeast,
    Southwest,
    Southeast,
}

impl Default for WinFloatAnchor {
    fn default() -> Self {
        Self::Northwest
    }
}

/// Message to be displayed in a grid.
#[derive(Debug, Copy, Clone)]
pub struct MsgSetPos<'a> {
    /// The grid to where the message should be displayed.
    pub grid: u64,
    /// In which row the message should be displayed.
    pub row: u64,
    /// Is the message scrolled further in the grid?
    pub scrolled: bool,
    /// Separation character for the message.
    pub sep_char: &'a str,
}

/// New view port of a window.
#[derive(Default, Debug, Copy, Clone)]
pub struct WinViewPort {
    /// The grid to be displayed in the window.
    pub grid: u64,
    /// The window's number.
    pub win: WinNr,
    /// The top row of the grid which is displayed in the window.
    pub topline: u64,
    /// The bottom row of the grid which is displayed in the window.
    ///
    /// This is set to one line more the line count of the buffer, if
    /// there are filler lines past the end.
    pub botline: u64,
    /// The cursor line in the grid.
    pub curline: u64,
    /// The cursor column in the grid.
    pub curcol: u64,
}

/// An opaque type for window numbers.
#[derive(Default, Debug, Copy, Clone)]
pub struct WinNr(u64);

impl WinNr {
    fn decode(raw: &mut &[u8]) -> io::Result<Self> {
        msg::read_ext_meta(raw)?;
        msg::read_u64(raw).map(Self)
    }
}

impl<'a> RedrawEvent<'a> {
    pub(super) fn decode(raw: &mut &'a [u8], arena: &'a Bump) -> io::Result<Vec<'a, Self>> {
        let n_events = msg::read_array_len(raw)?;
        log::trace!("Received batch of {} events", n_events);
        let mut events = Vec::with_capacity_in(n_events, arena);

        for _ in 0..n_events {
            Self::decode_single(raw, &mut events, arena)?;
        }

        Ok(events)
    }

    fn decode_single(
        raw: &mut &'a [u8],
        events: &mut Vec<'a, Self>,
        arena: &'a Bump,
    ) -> io::Result<()> {
        let event_len = msg::read_array_len(raw)?;
        // the first element is the type of event, the rest can be a batch of events.
        let n_events = event_len - 1;
        events.reserve(n_events);

        let event_type = msg::read_string(raw)?;
        log::trace!("event_type = {}, n_events = {}", event_type, n_events);

        match event_type {
            // global events.
            "set_title" => events.push(Self::decode_set_title(raw)?),
            "set_icon" => events.push(Self::decode_set_icon(raw)?),
            "mode_info_set" => {
                for _ in 0..n_events {
                    events.push(Self::decode_mode_info_set(raw, arena)?);
                }
            }
            "option_set" => {
                for _ in 0..n_events {
                    events.push(Self::decode_option_set(raw)?)
                }
            }
            "mode_change" => {
                for _ in 0..n_events {
                    events.push(Self::decode_mode_change(raw)?)
                }
            }
            "mouse_on" => {
                msg::ensure_parameters_count(0)?;
                events.push(Self::Mouse(true));
            }
            "mouse_off" => {
                msg::ensure_parameters_count(0)?;
                events.push(Self::Mouse(false));
            }
            "busy_start" => {
                msg::ensure_parameters_count(raw, 0)?;
                events.push(Self::Busy(true));
            }
            "busy_stop" => {
                msg::ensure_parameters_count(raw, 0)?;
                events.push(Self::Busy(false));
            }
            "flush" => {
                msg::ensure_parameters_count(raw, 0)?;
                events.push(Self::Flush);
            }
            // ignored global events.
            "suspend" | "update_menu" | "bell" | "visual_bell" => {
                msg::ensure_parameters_count(raw, 0)?;
            }

            // grid events
            "grid_resize" => {
                for _ in 0..n_events {
                    events.push(Self::decode_grid_resize(raw)?);
                }
            }
            "default_colors_set" => {
                for _ in 0..n_events {
                    events.push(Self::decode_default_colors_set(raw)?);
                }
            }
            "hl_attr_define" => {
                for _ in 0..n_events {
                    events.push(Self::decode_hl_attr_define(raw)?);
                }
            }
            "hl_group_set" => {
                for _ in 0..n_events {
                    events.push(Self::decode_hl_group_set(raw)?);
                }
            }
            "grid_line" => {
                for _ in 0..n_events {
                    events.push(Self::decode_grid_line(raw, arena)?);
                }
            }
            "grid_clear" => {
                for _ in 0..n_events {
                    events.push(Self::decode_grid_clear(raw)?);
                }
            }
            "grid_destroy" => {
                for _ in 0..n_events {
                    events.push(Self::decode_grid_destroy(raw)?);
                }
            }
            "grid_cursor_goto" => {
                events.push(Self::decode_grid_cursor_goto(raw)?);
            }
            "grid_scroll" => {
                for _ in 0..n_events {
                    events.push(Self::decode_grid_scroll(raw)?);
                }
            }

            // multigrid events
            "win_pos" => {
                for _ in 0..n_events {
                    events.push(Self::decode_win_pos(raw)?);
                }
            }
            "win_float_pos" => {
                for _ in 0..n_events {
                    events.push(Self::decode_win_float_pos(raw)?);
                }
            }
            "win_external_pos" => {
                for _ in 0..n_events {
                    events.push(Self::decode_win_external_pos(raw)?);
                }
            }
            "win_hide" => {
                for _ in 0..n_events {
                    events.push(Self::decode_win_hide(raw)?);
                }
            }
            "win_close" => {
                for _ in 0..n_events {
                    events.push(Self::decode_win_close(raw)?);
                }
            }
            "msg_set_pos" => {
                for _ in 0..n_events {
                    events.push(Self::decode_msg_set_pos(raw)?);
                }
            }
            "win_viewport" => {
                for _ in 0..n_events {
                    events.push(Self::decode_win_viewport(raw)?);
                }
            }
            _ => {
                log::warn!("received unknown event type");
                return msg::err_invalid_input();
            }
        }

        Ok(())
    }

    fn decode_set_title(raw: &mut &'a [u8]) -> io::Result<Self> {
        msg::ensure_parameters_count(raw, 1)?;

        Ok(Self::SetTitle(msg::read_string(raw)?))
    }

    fn decode_set_icon(raw: &mut &'a [u8]) -> io::Result<Self> {
        msg::ensure_parameters_count(raw, 1)?;

        Ok(Self::SetIcon(msg::read_string(raw)?))
    }

    fn decode_mode_info_set(raw: &mut &'a [u8], arena: &'a Bump) -> io::Result<Self> {
        msg::ensure_parameters_count(raw, 2)?;

        let cursor_style_enabled = msg::read_bool(raw)?;
        let n_infos = msg::read_array_len(raw)?;
        let mut mode_infos = Vec::with_capacity_in(n_infos, arena);

        for _ in 0..n_infos {
            let mut info = ModeInfo {
                cursor_shape: CursorShape::Block,
                cell_percentage: 0.0,
                attr_id: 0,
            };

            for _ in 0..msg::read_map_len(raw)? {
                match msg::read_string(raw)? {
                    "cursor_shape" => {
                        info.cursor_shape = match msg::read_string(raw)? {
                            "block" => CursorShape::Block,
                            "horizontal" => CursorShape::Horizontal,
                            "vertical" => CursorShape::Vertical,
                            _ => return msg::err_invalid_input(),
                        }
                    }
                    "cell_percentage" => {
                        info.cell_percentage = msg::read_u64(raw)? as f64 / 100.0
                    },
                    "attr_id" | "hl_id" => info.attr_id = msg::read_u64(raw)?,
                    // Ignored keys.
                    "blinkoff" | "blinkon" | "blinkwait" | "attr_id_lm" | "id_lm" | "mouse_shape" => {
                        msg::read_u64(raw)?;
                    }
                    "short_name" | "name" => {
                        msg::read_string(raw)?;
                    }
                    opt => {
                        log::error!("received invalid mode info option: {}", opt);
                        return msg::err_invalid_input();
                    },
                }
            }

            mode_infos.push(info)
        }

        Ok(Self::ModeInfoSet {
            cursor_style_enabled,
            mode_infos,
        })
    }

    fn decode_option_set(raw: &mut &'a [u8]) -> io::Result<Self> {
        msg::ensure_parameters_count(raw, 2)?;
        let option = msg::read_string(raw)?;
        log::trace!("received option = {}", option);

        match option {
            "ambiwidth" | "guifontwide" | "guifont" | "guifontset" => {
                return Ok(Self::OptionSet(UiOption::String {
                    option,
                    value: msg::read_string(raw)?,
                }));
            }
            "pumblend" | "showtabline" | "ttimeoutlen" | "linespace" => {
                return Ok(Self::OptionSet(UiOption::Int {
                    option,
                    value: msg::read_i64(raw)?,
                }));
            }
            "arabicshape" | "termguicolors" | "emoji" | "mousefocus" | "ttimeout"
            | "ext_linegrid" | "ext_multigrid" | "ext_hlstate" | "ext_termcolors"
            | "ext_cmdline" | "ext_popupmenu" | "ext_tabline" | "ext_wildmenu" | "ext_messages" => {
                return Ok(Self::OptionSet(UiOption::Bool {
                    option,
                    value: msg::read_bool(raw)?,
                }))
            }
            option => {
                log::warn!("found unknown option {}", option);
                return msg::err_invalid_input();
            }
        }
    }

    fn decode_mode_change(raw: &mut &'a [u8]) -> io::Result<Self> {
        msg::ensure_parameters_count(raw, 2)?;

        let _name = msg::read_string(raw)?;

        msg::read_u64(raw).map(Self::ModeChange)
    }

    fn decode_grid_resize(raw: &mut &'a [u8]) -> io::Result<Self> {
        msg::ensure_parameters_count(raw, 3)?;

        let grid = msg::read_u64(raw)?;
        let width = msg::read_u64(raw)?;
        let height = msg::read_u64(raw)?;

        Ok(Self::GridResize {
            grid,
            width,
            height,
        })
    }

    fn decode_default_colors_set(raw: &mut &'a [u8]) -> io::Result<Self> {
        msg::ensure_parameters_count(raw, 5)?;

        let foreground = msg::read_color(raw)?;
        let background = msg::read_color(raw)?;
        let special = msg::read_color(raw)?;

        msg::read_u64(raw)?;
        msg::read_u64(raw)?;

        Ok(Self::DefaultColorsSet(DefaultColorSet {
            foreground,
            background,
            special,
        }))
    }

    fn decode_hl_attr_define(raw: &mut &'a [u8]) -> io::Result<Self> {
        fn consume_attr<'b>(raw: &mut &'b [u8]) -> io::Result<RgbAttr> {
            let mut attr = RgbAttr::default();

            for _ in 0..msg::read_map_len(raw)? {
                match msg::read_string(raw)? {
                    "foreground" => attr.foreground = Some(msg::read_color(raw)?),
                    "background" => attr.background = Some(msg::read_color(raw)?),
                    "special" => attr.special = Some(msg::read_color(raw)?),
                    "blend" => attr.blend = msg::read_u64(raw)? as u8,
                    "reverse" => {
                        msg::read_bool(raw)?;
                        attr.flags.insert(RgbAttrFlags::REVERSE);
                    }
                    "italic" => {
                        msg::read_bool(raw)?;
                        attr.flags.insert(RgbAttrFlags::ITALIC);
                    }
                    "bold" => {
                        msg::read_bool(raw)?;
                        attr.flags.insert(RgbAttrFlags::BOLD);
                    }
                    "strikethrough" => {
                        msg::read_bool(raw)?;
                        attr.flags.insert(RgbAttrFlags::STRIKETHROUGH);
                    }
                    "underline" => {
                        msg::read_bool(raw)?;
                        attr.flags.insert(RgbAttrFlags::UNDERLINE);
                    }
                    "undercurl" => {
                        msg::read_bool(raw)?;
                        attr.flags.insert(RgbAttrFlags::UNDERCURL);
                    }
                    _ => return msg::err_invalid_input(),
                }
            }

            Ok(attr)
        }

        msg::ensure_parameters_count(raw, 4)?;

        let id = msg::read_u64(raw)?;
        let rgb_attr = consume_attr(raw)?;
        let _cterm_attr = consume_attr(raw)?;

        // consume info parameter.
        msg::ensure_parameters_count(raw, 0)?;

        Ok(Self::HlAttrDefine(HighlightAttr { id, rgb_attr }))
    }

    fn decode_hl_group_set(raw: &mut &'a [u8]) -> io::Result<Self> {
        msg::ensure_parameters_count(raw, 2)?;

        let name = msg::read_string(raw)?;
        let hl_id = msg::read_u64(raw)?;

        Ok(Self::HlGroupSet { name, hl_id })
    }

    fn decode_grid_line(raw: &mut &'a [u8], arena: &'a Bump) -> io::Result<Self> {
        msg::ensure_parameters_count(raw, 4)?;

        let grid = msg::read_u64(raw)?;
        let row = msg::read_u64(raw)?;
        let col_start = msg::read_u64(raw)?;
        let cells_len = msg::read_array_len(raw)?;
        let mut grid_line = GridLine {
            grid,
            row,
            col_start,
            cells: Vec::with_capacity_in(cells_len, arena),
        };

        let mut last_hl_id = 0;
        for _ in 0..cells_len {
            let cell_tuple_len = msg::read_array_len(raw)?;
            let mut cell = GridCell {
                text: "",
                hl_id: last_hl_id,
                repeated: 1,
            };
            cell.text = msg::read_string(raw)?;

            if cell_tuple_len > 1 {
                cell.hl_id = msg::read_u64(raw)?;
                last_hl_id = cell.hl_id;
            }

            if cell_tuple_len > 2 {
                cell.repeated = msg::read_u64(raw)?;
            }

            grid_line.cells.push(cell);
        }

        Ok(Self::GridLine(grid_line))
    }

    fn decode_grid_clear(raw: &mut &'a [u8]) -> io::Result<Self> {
        msg::ensure_parameters_count(raw, 1)?;

        Ok(Self::GridClear(msg::read_u64(raw)?))
    }

    fn decode_grid_destroy(raw: &mut &'a [u8]) -> io::Result<Self> {
        msg::ensure_parameters_count(raw, 1)?;

        Ok(Self::GridDestroy(msg::read_u64(raw)?))
    }

    fn decode_grid_cursor_goto(raw: &mut &'a [u8]) -> io::Result<Self> {
        msg::ensure_parameters_count(raw, 3)?;

        let mut goto = GridGoto::default();

        goto.grid = msg::read_u64(raw)?;
        goto.row = msg::read_u64(raw)?;
        goto.column = msg::read_u64(raw)?;

        Ok(Self::GridCursorGoto(goto))
    }

    fn decode_grid_scroll(raw: &mut &'a [u8]) -> io::Result<Self> {
        msg::ensure_parameters_count(raw, 7)?;

        let mut scroll = GridScroll::default();

        scroll.grid = msg::read_u64(raw)?;
        scroll.top = msg::read_u64(raw)?;
        scroll.bottom = msg::read_u64(raw)?;
        scroll.left = msg::read_u64(raw)?;
        scroll.right = msg::read_u64(raw)?;
        scroll.rows = msg::read_i64(raw)?;

        msg::read_u64(raw)?;

        Ok(Self::GridScroll(scroll))
    }

    fn decode_win_pos(raw: &mut &'a [u8]) -> io::Result<Self> {
        msg::ensure_parameters_count(raw, 6)?;

        let mut win_pos = WinPos::default();
        win_pos.grid = msg::read_u64(raw)?;
        win_pos.win = WinNr::decode(raw)?;
        win_pos.start_row = msg::read_u64(raw)?;
        win_pos.start_col = msg::read_u64(raw)?;
        win_pos.width = msg::read_u64(raw)?;
        win_pos.height = msg::read_u64(raw)?;

        Ok(Self::WinPos(win_pos))
    }

    fn decode_win_float_pos(raw: &mut &'a [u8]) -> io::Result<Self> {
        msg::ensure_parameters_count(raw, 7)?;

        let mut win_float_pos = WinFloatPos::default();
        win_float_pos.grid = msg::read_u64(raw)?;
        win_float_pos.win = WinNr::decode(raw)?;

        win_float_pos.anchor = match msg::read_string(raw)? {
            "NW" => WinFloatAnchor::Northwest,
            "NE" => WinFloatAnchor::Northeast,
            "SW" => WinFloatAnchor::Southwest,
            "SE" => WinFloatAnchor::Southeast,
            _ => unreachable!("received unknown anchor"),
        };

        win_float_pos.anchor_grid = msg::read_u64(raw)?;
        win_float_pos.anchor_row = msg::read_u64(raw)?;
        win_float_pos.anchor_col = msg::read_u64(raw)?;
        win_float_pos.focusable = msg::read_bool(raw)?;

        Ok(Self::WinFloatPos(win_float_pos))
    }

    fn decode_win_external_pos(raw: &mut &'a [u8]) -> io::Result<Self> {
        msg::ensure_parameters_count(raw, 2)?;

        let grid = msg::read_u64(raw)?;
        let win = WinNr::decode(raw)?;

        Ok(Self::WinExternalPos { grid, win })
    }

    fn decode_win_hide(raw: &mut &'a [u8]) -> io::Result<Self> {
        msg::ensure_parameters_count(raw, 1)?;

        Ok(Self::WinHide(WinNr::decode(raw)?))
    }

    fn decode_win_close(raw: &mut &'a [u8]) -> io::Result<Self> {
        msg::ensure_parameters_count(raw, 1)?;

        Ok(Self::WinClose(WinNr::decode(raw)?))
    }

    fn decode_msg_set_pos(raw: &mut &'a [u8]) -> io::Result<Self> {
        msg::ensure_parameters_count(raw, 4)?;

        let grid = msg::read_u64(raw)?;
        let row = msg::read_u64(raw)?;
        let scrolled = msg::read_bool(raw)?;
        let sep_char = msg::read_string(raw)?;

        Ok(Self::MsgSetPos(MsgSetPos {
            grid,
            row,
            scrolled,
            sep_char,
        }))
    }

    fn decode_win_viewport(raw: &mut &'a [u8]) -> io::Result<Self> {
        msg::ensure_parameters_count(raw, 6)?;

        let mut viewport = WinViewPort::default();
        viewport.grid = msg::read_u64(raw)?;
        viewport.win = WinNr::decode(raw)?;
        viewport.topline = msg::read_u64(raw)?;
        viewport.botline = msg::read_u64(raw)?;
        viewport.curline = msg::read_u64(raw)?;
        viewport.curcol = msg::read_u64(raw)?;

        Ok(Self::WinViewPort(viewport))
    }
}

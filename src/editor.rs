use crate::grid::*;
use crate::neovim::*;
use cache_padded::CachePadded;
use font_kit::family_handle::FamilyHandle;
use font_kit::source::*;
use std::sync::{
    atomic::{AtomicBool, AtomicI64, AtomicU16, Ordering},
    Arc,
};
use tokio::sync::mpsc::{Receiver, Sender, error::TrySendError};

pub use self::buffering::*;

const MPSC_CHANNEL_BUFFER_SIZE: usize = 128;

pub struct Editor {
    lines: TripleBufferWriter,
    modes: Vec<ModeInfo>,
    curr_mode: usize,
    shared_state: Arc<UiEditorSharedState>,
    font_source: SystemSource,
    tx: Sender<UiEditorEvent>,
}

impl EventListener for Editor {
    fn on_redraw_event(&mut self, event: RedrawEvent<'_>) {
        match event {
            // Global events
            RedrawEvent::ModeInfoSet { cursor_style_enabled, mode_infos } => {
                self.set_modes_info(&mode_infos, cursor_style_enabled);
            }
            RedrawEvent::OptionSet(option) => self.set_ui_option(option),
            RedrawEvent::ModeChange(new_mode) => self.change_mode(new_mode),
            RedrawEvent::Mouse(mouse_enabled) => self.set_mouse(mouse_enabled),
            RedrawEvent::Flush => self.flush(),

            // Grid events
            RedrawEvent::GridResize { width, height, .. } => self.resize_grid(width, height),
            RedrawEvent::DefaultColorsSet(new_default) => self.set_default_color_set(new_default),
            RedrawEvent::HlAttrDefine(hl_attr) => self.define_hl_attr(hl_attr),
            RedrawEvent::GridLine(line) => self.redraw_grid_line(line),
            RedrawEvent::GridClear(_) => self.clear_grid(),
            RedrawEvent::GridScroll(scroll) => self.scroll_grid(scroll),

            // Ignore rest of events.
            _ => {}
        }
    }
}

impl Editor {
    pub fn new() -> (Self, UiStateFromEditor) {
        let (lines, output) = buffering::new_triple_buffer();
        let shared_state = Arc::<UiEditorSharedState>::default();
        let (tx, rx) = tokio::sync::mpsc::channel(MPSC_CHANNEL_BUFFER_SIZE);

        let editor = Self {
            lines,
            modes: Vec::default(),
            curr_mode: 0,
            font_source: SystemSource::new(),
            shared_state: shared_state.clone(),
            tx,
        };

        let ui_state = UiStateFromEditor {
            reader: output,
            shared: shared_state,
            recv: rx,
        };

        (editor, ui_state)
    }

    pub fn set_modes_info(&mut self, modes: &[ModeInfo], cursor_style_enabled: bool) {
        self.modes.clear();
        self.modes.extend_from_slice(modes);

        self.shared_state
            .cursor_style_enabled
            .store(cursor_style_enabled, Ordering::Release);
    }

    pub fn set_ui_option(&mut self, option: UiOption<'_>) {
        match option {
            UiOption::Int { option, value } => {
                if option == "linespace" {
                    self.shared_state.linespace.store(value, Ordering::Release);
                }
            }
            UiOption::String { option, value } => match option {
                "guifont" | "guifontset" => {
                    if let Some((font_family, size)) = self.parse_guifont(value) {
                        self.send_event(UiEditorEvent::FontChanged(font_family, size));
                    } else {
                        log::warn!("No available font in guifont");
                    }
                }
                _ => {}
            }
            UiOption::Bool { option, value } => match option {
                "ext_linegrid" => self
                    .shared_state
                    .set_ui_option_if(UiOptions::EXT_LINEGRID, value),
                "ext_multigrid" => self
                    .shared_state
                    .set_ui_option_if(UiOptions::EXT_MULTIGRID, value),
                "ext_hlstate" => self
                    .shared_state
                    .set_ui_option_if(UiOptions::EXT_HLSTATE, value),
                "ext_termcolors" => self
                    .shared_state
                    .set_ui_option_if(UiOptions::EXT_TERMCOLORS, value),
                "ext_cmdline" => self
                    .shared_state
                    .set_ui_option_if(UiOptions::EXT_LINEGRID, value),
                "ext_popupmenu" => self
                    .shared_state
                    .set_ui_option_if(UiOptions::EXT_POPUPMENU, value),
                "ext_tabline" => self
                    .shared_state
                    .set_ui_option_if(UiOptions::EXT_TABLINE, value),
                "ext_messages" => self
                    .shared_state
                    .set_ui_option_if(UiOptions::EXT_MESSAGES, value),
                "mousefocus" => self
                    .shared_state
                    .mouse_focus_enabled
                    .store(value, Ordering::Release),
                _ => {}
            },
        }
    }

    fn parse_guifont(&self, guifont: &str) -> Option<(FamilyHandle, u8)> {
        dbg!(guifont);
        // TODO: handle escaped commas
        for font in guifont.split(',') {
            dbg!(font);
            if let Some((name, size)) = font.split_once(":h") {
                let size = size.parse::<u8>().unwrap_or(12);

                if let Ok(font_family) = self.font_source.select_family_by_name(name) {
                    return Some((font_family, size));
                }
            }
        }

        None
    }

    pub fn change_mode(&mut self, mode_idx: u64) {
        if (mode_idx as usize) < self.modes.len() {
            self.curr_mode = mode_idx as usize;
        }
    }

    pub fn set_mouse(&self, enabled: bool) {
        self.shared_state
            .mouse_enabled
            .store(enabled, Ordering::Release);
    }

    pub fn set_buzy(&self, buzy: bool) {
        self.shared_state.buzy.store(buzy, Ordering::Release);
    }

    pub fn flush(&mut self) {
        self.lines.buffer().render();

        // Check if the UI already processed the previous completed buffer.
        //
        // If it haven't, there is a pending redraw event in flight, meaning
        // that we will duplicate the rendering work if we send another one.
        if !self.lines.publish() {
            self.send_event(UiEditorEvent::Redraw);
        }
    }

    pub fn resize_grid(&mut self, width: u64, height: u64) {
        self.lines.buffer().resize(height as usize, width as usize);
    }

    pub fn set_default_color_set(&mut self, color_set: DefaultColorSet) {
        self.send_event(UiEditorEvent::SetDefaultColorsSet(color_set));
    }

    pub fn define_hl_attr(&mut self, hl_attr: HighlightAttr) {
        self.send_event(UiEditorEvent::DefineHlAttr(hl_attr));
    }

    pub fn redraw_grid_line(&mut self, grid_line: GridLine<'_>) {
        self.lines.buffer().update_line(grid_line);
    }

    pub fn clear_grid(&mut self) {
        self.lines.buffer().clear();
    }

    pub fn move_cursor(&mut self, cursor_goto: GridGoto) {
        self.lines
            .buffer()
            .cursor_mut()
            .move_to(cursor_goto.row as usize, cursor_goto.column as usize);
    }

    pub fn scroll_grid(&mut self, scroll: GridScroll) {
        self.lines.buffer().scroll(scroll);
    }

    fn send_event(&mut self, mut event: UiEditorEvent) {
        // We can spin loop as the chances of the UI be slow enough to not
        // keep track of the small amount of events that we send is low.
        //
        // The only time where we send many events at once is when defining
        // highlight groups, but storing new groups is very quickly.
        loop {
            if let Err(err) = self.tx.try_send(event) {
                if let TrySendError::Full(e) = err {
                    event = e;
                    std::sync::atomic::spin_loop_hint();
                    continue
                } else {
                    panic!("UI event receiver dropped before sender")
                }
            }

            break;
        }
    }
}

pub enum UiEditorEvent {
    FontChanged(FamilyHandle, u8),
    SetDefaultColorsSet(DefaultColorSet),
    DefineHlAttr(HighlightAttr),
    Redraw,
}

impl std::fmt::Debug for UiEditorEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FontChanged(_, size) => f.debug_tuple("UiEditorEvent::FontChanged")
                .field(&"..")
                .field(&size)
                .finish(),
            Self::SetDefaultColorsSet(set) => f.debug_tuple("UiEditorEvent::SetDefaultColorsSet")
                .field(&set)
                .finish(),
            Self::DefineHlAttr(hl_attr) => f.debug_tuple("UiEditorEvent::DefineHlAttr")
                .field(&hl_attr)
                .finish(),
            Self::Redraw => f.debug_tuple("UiEditorEvent::Redraw").finish(),
        }
    }
}

#[derive(Debug, Default)]
pub struct HighlightGroups {
    groups: Vec<RgbAttr>,
    default: RgbAttr,
}

impl HighlightGroups {
    fn update_default(&mut self, default: RgbAttr) {
        self.default = default;
    }

    fn update(&mut self, hl_attrs: Vec<HighlightAttr>) {
        self.groups.reserve(hl_attrs.len());

        for hl in hl_attrs {
            if self.groups.len() <= hl.id as usize {
                self.groups.resize_with(hl.id as usize, Default::default);
            }

            self.groups[hl.id as usize] = hl.rgb_attr;
        }
    }

    /// Returns the final [`RgbAttr`] to be used for a highlight group.
    ///
    /// Already handles the logic behind the default grouping and reverse colors.
    pub fn group_color_set(&self, hl_id: u64) -> RgbAttr {
        if let Some(mut hl) = self.groups.get(hl_id as usize).copied() {
            hl.foreground = hl.foreground.or(self.default.foreground);
            hl.background = hl.background.or(self.default.background);
            hl.special = hl.special.or(self.default.special);

            if hl.reverse() {
                hl = hl.reverse_rgb_attr();
            }

            hl
        } else {
            self.default
        }
    }

    /// Returns the final [`RgbAttr`] to be used when rendering the cursor position.
    pub fn cursor_color_set(&self, hl_id: u64) -> RgbAttr {
        self.group_color_set(hl_id).reverse_rgb_attr()
    }
}

pub struct UiStateFromEditor {
    pub reader: TripleBufferReader,
    pub shared: Arc<UiEditorSharedState>,
    pub recv: Receiver<UiEditorEvent>,
}

#[derive(Debug, Default)]
pub struct UiEditorSharedState {
    ui_options: CachePadded<AtomicU16>,
    cursor_style_enabled: CachePadded<AtomicBool>,
    mouse_focus_enabled: CachePadded<AtomicBool>,
    mouse_enabled: CachePadded<AtomicBool>,
    linespace: CachePadded<AtomicI64>,
    buzy: CachePadded<AtomicBool>,
}

impl UiEditorSharedState {
    pub fn ui_options(&self) -> UiOptions {
        UiOptions::from_bits(self.ui_options.load(Ordering::Relaxed))
            .expect("invalid ui_options bits")
    }

    fn set_ui_option_if(&self, ui_option: UiOptions, flag: bool) {
        if flag {
            self.ui_options
                .fetch_or(ui_option.bits(), Ordering::Release);
        } else {
            self.ui_options
                .fetch_and(!ui_option.bits(), Ordering::Release);
        }
    }

    pub fn cursor_style_enabled(&self) -> bool {
        self.cursor_style_enabled.load(Ordering::Relaxed)
    }

    pub fn mouse_focus_enabled(&self) -> bool {
        self.mouse_focus_enabled.load(Ordering::Relaxed)
    }

    pub fn mouse_enabled(&self) -> bool {
        self.mouse_enabled.load(Ordering::Relaxed)
    }

    pub fn buzy(&self) -> bool {
        self.buzy.load(Ordering::Relaxed)
    }
}

mod buffering {
    //! # Triple Buffering for Grids
    //!
    //! This is a simplified version of [`triple-buffer`](https://github.com/HadrienG2/triple-buffer/),
    //! specific for our needs when flushing grid updates.
    //!
    //! We can't use the original crate as it doesn't give access to the back-buffer directly,
    //! which we need to ensure that new writing buffer is updated with the latest changes.
    use super::Lines;
    use cache_padded::CachePadded;
    use std::cell::UnsafeCell;
    use std::sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    };

    const BACK_INDEX_MASK: u8 = 0b11;
    const BACK_DIRTY_BIT: u8 = 0b100;

    struct TripleBufferSharedState {
        buffers: [CachePadded<UnsafeCell<Lines>>; 3],
        back_info: CachePadded<AtomicU8>,
    }

    impl TripleBufferSharedState {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                buffers: Default::default(),
                back_info: CachePadded::new(AtomicU8::new(0)),
            })
        }

        fn updated(&self) -> bool {
            self.back_info.load(Ordering::Relaxed) & BACK_DIRTY_BIT != 0
        }
    }

    pub fn new_triple_buffer() -> (TripleBufferWriter, TripleBufferReader) {
        let state = TripleBufferSharedState::new();

        let writer = TripleBufferWriter {
            shared: state.clone(),
            input_idx: 1,
        };

        let reader = TripleBufferReader {
            shared: state,
            output_idx: 2,
        };

        (writer, reader)
    }

    /// Writer part of the triple buffer.
    pub struct TripleBufferWriter {
        shared: Arc<TripleBufferSharedState>,
        input_idx: u8,
    }

    unsafe impl Send for TripleBufferWriter {}
    unsafe impl Sync for TripleBufferWriter {}

    impl TripleBufferWriter {
        /// The current writing buffer.
        pub fn buffer(&mut self) -> &mut Lines {
            let input_ptr = self.shared.buffers[self.input_idx as usize].get();

            // SAFETY: This is safe because the synchronization protocol ensures
            // that we have exclusive access to this buffer.
            unsafe { &mut *input_ptr }
        }

        /// Publish the changes made to the reader.
        pub fn publish(&mut self) -> bool {
            let old_index = self.input_idx;
            // Swap the input buffer and the back buffer, setting the dirty bit
            let former_back_info = self
                .shared
                .back_info
                .swap(self.input_idx | BACK_DIRTY_BIT, Ordering::AcqRel);

            // The old back buffer becomes our new input buffer.
            self.input_idx = former_back_info & BACK_INDEX_MASK;

            // SAFETY: This is safe as we're creating a immutable reference and the protocol
            //   ensures that the reader can't modify the complete buffer.
            let complete = unsafe { &*self.shared.buffers[old_index as usize].get() };
            let writing = self.buffer();

            // Ensure that the writing buffer is up to date with the complete buffer.
            writing.clone_from(complete);

            // Tell whether we have overwritten unread data.
            former_back_info & BACK_DIRTY_BIT != 0
        }
    }

    pub struct TripleBufferReader {
        shared: Arc<TripleBufferSharedState>,
        output_idx: u8,
    }

    unsafe impl Send for TripleBufferReader {}
    unsafe impl Sync for TripleBufferReader {}

    impl TripleBufferReader {
        pub fn buffer(&mut self) -> &Lines {
            self.update();
            let output_ptr = self.shared.buffers[self.output_idx as usize].get();

            // SAFETY: This is safe as he protocol ensures that we've exclusive access
            //   to this buffer.
            unsafe { &*output_ptr }
        }

        fn update(&mut self) {
            let shared = &*self.shared;

            // Check if an update is present in the back-buffer.
            if shared.updated() {
                // If so, exchange our output buffer with the back-buffer, thusly
                // acquiring exclusive access to the old back buffer while giving
                // the producer a new back-buffer to write to.
                let former_back_info = shared.back_info.swap(self.output_idx, Ordering::AcqRel);

                // make the old back-buffer our new output buffer.
                self.output_idx = former_back_info & BACK_INDEX_MASK;
            }
        }
    }
}

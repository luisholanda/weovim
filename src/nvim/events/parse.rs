use super::grid::*;
use super::*;
use neovim_lib::Value;

macro_rules! unwrap_array {
    ($v: expr) => {
        match $v {
            Value::Array(a) => a,
            e => panic!("Expected array, received {}", e),
        }
    };
}

macro_rules! unwrap_string {
    ($v: expr) => {
        match $v {
            Value::String(s) => s.into_str().unwrap_or_else(String::new),
            e => panic!("Expected string, received {}", e),
        }
    };
}

macro_rules! unwrap_map {
    ($v: expr) => {
        match $v {
            Value::Map(m) => m,
            e => panic!("Expected map, received {}", e),
        }
    };
}

macro_rules! unwrap_usize {
    ($v: expr) => {
        match $v {
            Value::Integer(ref i) => match i.as_u64() {
                Some(u) => u as usize,
                None => panic!("Expected usize found {}", i),
            },
            e => panic!("Expected number, received {}", e),
        }
    };
}

pub fn redrawcmd(mut args: Vec<Value>) -> Option<RedrawEvent> {
    let mut drain = args.drain(..);

    match drain.next()?.as_str()? {
        "set_title" => Some(RedrawEvent::SetTitle(unwrap_string!(drain.next()?))),
        "mode_info_set" => parse_mode_info(unwrap_array!(drain.next()?)),
        "option_set" => parse_uioptions(drain),
        "mode_change" => parse_mode_change(unwrap_array!(drain.next()?)),
        "busy_start" => Some(RedrawEvent::Busy(true)),
        "busy_stop" => Some(RedrawEvent::Busy(false)),
        "flush" => Some(RedrawEvent::Flush),
        "grid_resize" => {
            let mut goto = unwrap_array!(drain.next()?);

            let height = unwrap_usize!(goto.pop()?);
            let width = unwrap_usize!(goto.pop()?);
            let grid = unwrap_usize!(goto.pop()?);

            Some(RedrawEvent::GridResize {
                grid,
                width,
                height,
            })
        }
        "hl_attr_define" => parse_hl_attr_define(drain),
        "grid_line" => parse_grid_line(drain),
        "default_colors_set" => parse_default_colors(drain),
        "grid_clear" => Some(RedrawEvent::GridClear),
        "grid_destroy" => Some(RedrawEvent::GridDestroy),
        "grid_cursor_goto" => {
            let mut goto = unwrap_array!(drain.next()?);

            let column = unwrap_usize!(goto.pop()?);
            let row = unwrap_usize!(goto.pop()?);
            let grid = unwrap_usize!(goto.pop()?);

            Some(RedrawEvent::GridCursorGoto(GridGoto { grid, row, column }))
        }
        "grid_scroll" => {
            let mut goto = unwrap_array!(drain.next()?);

            let columns = unwrap_usize!(goto.pop()?);
            let rows = goto.pop()?.as_i64()?;
            let right = unwrap_usize!(goto.pop()?);
            let left = unwrap_usize!(goto.pop()?);
            let bottom = unwrap_usize!(goto.pop()?);
            let top = unwrap_usize!(goto.pop()?);
            let grid = unwrap_usize!(goto.pop()?);

            Some(RedrawEvent::GridScroll(GridScroll {
                grid,
                top,
                bottom,
                left,
                right,
                rows,
                columns,
            }))
        }
        _ => None,
    }
}

fn parse_mode_info(args: Vec<Value>) -> Option<RedrawEvent> {
    let mut args = args.into_iter();
    let csen = args.next()?.as_bool()?;

    let info = unwrap_array!(args.next()?)
        .into_iter()
        .filter_map(|a| {
            let mut info = ModeInfo::default();

            for (k, v) in unwrap_map!(a) {
                match k.as_str()? {
                    "blinkon" => info.blink_on = v.as_u64()?,
                    "blinkoff" => info.blink_off = v.as_u64()?,
                    "blinkwait" => info.blink_wait = v.as_u64()?,
                    "cell_percentage" => info.cell_percentage = v.as_f64()?,
                    "cursor_shape" => {
                        info.cursor_shape = match v.as_str()? {
                            "block" => CursorShape::Block,
                            "horizontal" => CursorShape::Horizontal,
                            "vertical" => CursorShape::Vertical,
                            _ => return None,
                        }
                    }
                    "name" => info.name = unwrap_string!(v),
                    "short_name" => info.short_name = unwrap_string!(v),
                    _ => {}
                }
            }

            Some(info)
        })
        .collect();

    Some(RedrawEvent::ModeInfoSet {
        cursor_style_enabled: csen,
        mode_info: info,
    })
}

fn parse_uioptions(args: impl Iterator<Item = Value>) -> Option<RedrawEvent> {
    let options = args
        .filter_map(|a| {
            let mut a = unwrap_array!(a).into_iter();

            match a.next()?.as_str()? {
                "guifont" => Some(UiOption::GuiFont(unwrap_string!(a.next()?))),
                "guifontwide" => Some(UiOption::GuiFontWide(unwrap_string!(a.next()?))),
                "linespace" => Some(UiOption::LineSpace(a.next()?.as_i64()?)),
                _ => None,
            }
        })
        .collect();

    Some(RedrawEvent::OptionSet(options))
}

fn parse_mode_change(args: Vec<Value>) -> Option<RedrawEvent> {
    let mut args = args.into_iter();
    let name = unwrap_string!(args.next()?);
    let index = unwrap_usize!(args.next()?);

    Some(RedrawEvent::ModeChange { name, index })
}

fn parse_default_colors(mut args: impl Iterator<Item = Value>) -> Option<RedrawEvent> {
    args.next()?.as_array().map(|colors| {
        let fg = Color::from_rgb_u64(colors[0].as_u64().unwrap_or(0));
        let bg = Color::from_rgb_u64(colors[1].as_u64().unwrap_or(std::u64::MAX));
        let sp = Color::from_rgb_u64(colors[2].as_u64().unwrap_or(16_711_680));

        RedrawEvent::DefaultColorSet { fg, bg, sp }
    })
}

fn parse_hl_attr_define(args: impl Iterator<Item = Value>) -> Option<RedrawEvent> {
    let attrs = args
        .filter_map(|arg| {
            let mut arg = unwrap_array!(arg);
            if !arg.pop()?.is_array() {
                return None;
            }

            let cterm_attr = parse_rgb_attr(arg.pop()?)?;
            let rgb_attr = parse_rgb_attr(arg.pop()?)?;

            let id = arg.pop()?.as_u64()? as usize;

            Some(HighlightAttr {
                id,
                rgb_attr,
                cterm_attr,
            })
        })
        .collect();

    Some(RedrawEvent::HlAttrDefine(attrs))
}

fn parse_rgb_attr(arg: Value) -> Option<RgbAttr> {
    let mut attr = RgbAttr::default();

    for (k, v) in unwrap_map!(arg) {
        match k.as_str()? {
            "foreground" => attr.foreground = v.as_u64().map(Color::from_rgb_u64),
            "background" => attr.background = v.as_u64().map(Color::from_rgb_u64),
            "special" => attr.special = v.as_u64().map(Color::from_rgb_u64),
            "reverse" => attr.reverse = v.as_bool()?,
            "italic" => attr.italic = v.as_bool()?,
            "bold" => attr.bold = v.as_bool()?,
            "underline" => attr.underline = v.as_bool()?,
            "undercurl" => attr.undercurl = v.as_bool()?,
            _ => {}
        }
    }

    Some(attr)
}

fn parse_grid_line(lines: impl Iterator<Item = Value>) -> Option<RedrawEvent> {
    let grid_lines = lines
        .filter_map(|line| {
            let mut line = unwrap_array!(line);

            let line_cells = unwrap_array!(line.pop()?);

            let mut cells: Vec<GridCell> = Vec::with_capacity(line_cells.len());

            for lc in line_cells {
                let mut cell = unwrap_array!(lc).into_iter();

                let text = unwrap_string!(cell.next()?);
                let hl_id = cell
                    .next()
                    .and_then(|h| h.as_u64())
                    .unwrap_or_else(|| cells.last().unwrap().hl_id as u64)
                    as usize;
                let repeated = cell.next().and_then(|r| r.as_u64()).unwrap_or(1) as usize;

                cells.push(GridCell {
                    text,
                    hl_id,
                    repeated,
                });
            }

            let col_start = line.pop()?.as_u64()? as usize;
            let row = line.pop()?.as_u64()? as usize;
            let grid = line.pop()?.as_u64()? as usize;

            Some(GridLine {
                grid,
                row,
                col_start,
                cells,
            })
        })
        .collect();

    Some(RedrawEvent::GridLine(grid_lines))
}

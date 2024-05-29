#![feature(macro_metavar_expr, assert_matches)]

use {
    crate::extcmd::UnkCmd, charsets::Button, extcmd::ExtCmd, num_enum::TryFromPrimitive,
    std::ops::ControlFlow,
};

mod charsets;
mod extcmd;
mod imm;

/// The size of a dialog buffer
pub const BUFFER_SIZE: usize = 1024;

#[derive(Debug)]
enum Line {
    Text(LineText),
    BubbleBreak,
}

struct EventsToLinesOut {
    lines: Vec<Line>,
    start_scroll: u32,
    bubble_style: u8,
}

#[derive(Debug)]
struct LineText {
    text: String,
    hoffset: u8,
}

fn events_to_lines(events: Vec<imm::Event>) -> EventsToLinesOut {
    let mut lines = Vec::new();
    let mut linebuf = String::new();
    let mut hoffset = 0;
    let mut start_scroll = 0;
    let mut bubble_style = 0;
    for event in events {
        match event {
            imm::Event::Char(ch) => {
                linebuf.push(ch);
            }
            imm::Event::Newline => {
                lines.push(Line::Text(LineText {
                    text: std::mem::take(&mut linebuf),
                    hoffset,
                }));
            }
            imm::Event::Space | imm::Event::Tab => linebuf.push('\u{3000}'),
            imm::Event::NextBubble => lines.push(Line::BubbleBreak),
            imm::Event::BubbleStyle(style) => bubble_style = style,
            imm::Event::ExtCmd1D(_)
            | imm::Event::ExtStoreColor
            | imm::Event::ExtLoadColor
            | imm::Event::ExtSetColor(_)
            | imm::Event::ExtCmd06(..)
            | imm::Event::ExtCmd0B(..)
            | imm::Event::ExtCmd0C(..)
            | imm::Event::TextEffectBowserLaugh
            | imm::Event::TextEffectDarkStar
            | imm::Event::TextEffectNoise(..)
            | imm::Event::TextEffectStar(..)
            | imm::Event::TextEffectQuickPulse
            | imm::Event::TextEffectRainbow1
            | imm::Event::TextEffectRainbow2
            | imm::Event::TextEffectShadow
            | imm::Event::TextEffectShaky1
            | imm::Event::TextEffectShaky2(..)
            | imm::Event::TextEffectWavePulse
            | imm::Event::TextEffectWavy1
            | imm::Event::TextEffectWavy2 => {}
            imm::Event::Btn(btn) => {
                let str = match btn {
                    Button::A => "[A]",
                    Button::B => "[B]",
                    Button::Start => "[START]",
                    Button::CDown => "[C⬇]",
                    Button::CLeft => "[C◀]",
                    Button::Z => "[Z]",
                };
                linebuf.push_str(str);
            }
            imm::Event::ExtTextHoffset(off) => hoffset = off,
            imm::Event::ExtExtVOffset(off) => start_scroll = off as u32,
            _ => linebuf.push_str(&format!(" ( {event:02X?}) ")),
        }
    }
    // Push any leftovers
    let leftover = std::mem::take(&mut linebuf);
    if !leftover.is_empty() {
        lines.push(Line::Text(LineText {
            text: leftover,
            hoffset,
        }));
    }
    EventsToLinesOut {
        lines,
        start_scroll,
        bubble_style,
    }
}

pub struct DecodeImmBufOut {
    pub text: String,
    pub hoffs: u8,
}

pub fn decode_imm_buf(data: &[u8], mut scroll: u32) -> DecodeImmBufOut {
    let mut buf = String::new();
    let mut hoffs = 0;
    let out = events_to_lines(imm::decode_events(data));
    scroll += out.start_scroll;
    if out.bubble_style == 0x07 {
        scroll = scroll.saturating_sub(12);
    }
    let line_offs = (scroll / 16) as usize;
    let mut lines = out.lines.into_iter();
    // Skip line_offs lines
    let mut skipped = 0;
    while skipped < line_offs {
        match lines.next() {
            Some(Line::Text(LineText { text: _, hoffset })) => {
                skipped += 1;
                hoffs = hoffset;
            }
            Some(Line::BubbleBreak) => {}
            None => break,
        }
    }
    let mut shown = 0;
    for line in lines {
        match line {
            Line::Text(text) => {
                buf.push_str(&text.text);
                shown += 1;
                if shown == 3 {
                    break;
                }
                buf.push('\n');
            }
            Line::BubbleBreak => {
                if shown > 0 {
                    break;
                }
            }
        }
    }
    DecodeImmBufOut { text: buf, hoffs }
}

#[derive(Debug)]
pub enum Event {
    StyleChange(Style),
    Space,
    Dialog(String),
    End,
    Linebreak,
    Delay(u8),
    Bell,
    NextBubble,
    /// Sparkly text effect (for the starfolk)
    Sparkly,
    ButtonRef {
        button: Option<Button>,
        rawcode: u8,
    },
    ExtCmd(extcmd::ExtCmd),
    ExtCmdError {
        id: u8,
        argc: u8,
        args_got: u8,
    },
}

#[derive(Debug, TryFromPrimitive)]
#[repr(u8)]
pub enum Style {
    Invalid = 0x00,
    BubbleRight = 0x01,
    BubbleLeft = 0x02,
    BubbleA = 0x03,
    BubbleB = 0x04,
    WhiteBorder = 0x05,
    NarrationA = 0x06,
    SignPost = 0x07,
    BlueMessage = 0x08,
    Invalid2 = 0x09,
    WhiteBubbleA = 0x0A,
    WhiteBubbleB = 0x0B,
    NoDisplay = 0x0C,
    NarrationSilent = 0x0D,
    NoDisplayVCenter = 0x0E,
    NarrationB = 0x0F,
}

enum Status {
    Init,
    Style,
    Delay,
    ExtCmd,
    ExtCmdParams { id: u8, argc: u8, argidx: u8 },
}

pub fn to_string_nth_bubble(raw: &[u8], bubble_idx: u8) -> Result<String, String> {
    let mut s = String::new();
    let mut current_idx = 0;
    for event in translate(raw)? {
        if bubble_idx == current_idx {
            if let Event::NextBubble = &event {
                return Ok(s);
            }
            if let ControlFlow::Break(_) = write_event_string(&event, &mut s) {
                break;
            }
        } else if let Event::NextBubble = &event {
            current_idx += 1;
        }
    }
    Ok(s)
}

pub fn to_string(raw: &[u8]) -> Result<String, String> {
    let mut s = String::new();
    for event in translate(raw)? {
        if let ControlFlow::Break(_) = write_event_string(&event, &mut s) {
            break;
        }
    }
    Ok(s)
}

#[must_use]
fn write_event_string(event: &Event, s: &mut String) -> ControlFlow<()> {
    match event {
        Event::StyleChange(_) => {}
        // Ideographic (full width) space
        Event::Space => s.push('\u{3000}'),
        Event::Dialog(str) => s.push_str(str),
        Event::End => return ControlFlow::Break(()),
        Event::Linebreak => s.push('\n'),
        Event::Delay(_) => {}
        Event::Bell => {}
        Event::ButtonRef { button, rawcode } => match button {
            Some(b) => s.push_str(match b {
                Button::A => "[A]",
                Button::B => "[B]",
                Button::Start => "[START]",
                Button::CDown => "[C⬇]",
                Button::CLeft => "[C◀]",
                Button::Z => "[Z]",
            }),
            None => s.push_str(&format!("{{buttonref:{rawcode:02X}}}")),
        },
        Event::NextBubble => s.push_str("⭐\n"),
        Event::Sparkly => {}
        Event::ExtCmd(cmd) => match cmd {
            ExtCmd::GraphicsB { .. }
            | ExtCmd::Unk8 { .. }
            | ExtCmd::Unk13 { .. }
            | ExtCmd::Unk14 { .. }
            | ExtCmd::Unk29 { .. }
            | ExtCmd::LoadTextColor { .. }
            | ExtCmd::SaveTextColor { .. }
            | ExtCmd::TextColor { .. }
            | ExtCmd::StartEffect { .. }
            | ExtCmd::EndEffect { .. }
            | ExtCmd::Voice { .. }
            | ExtCmd::AutoScroll { .. }
            | ExtCmd::FontSize { .. }
            | ExtCmd::FontSizeReset { .. } => {}
            _ => s.push_str(&format!(" ( {cmd:?} ) ")),
        },
        Event::ExtCmdError { id, argc, args_got } => {
            s.push_str(&format!(
                "[extcmd_error] id: 0x{id:02X}, argc: {argc}, got: {args_got}"
            ));
        }
    }
    ControlFlow::Continue(())
}

enum LookupTable {
    Kana,
    Kanji,
    Latin,
    Button,
}

pub fn translate(raw: &[u8]) -> Result<Vec<Event>, String> {
    let mut events = Vec::new();
    let mut buf = String::new();
    let mut status = Status::Init;
    let mut lookup_table = LookupTable::Kana;
    let mut argbuf = Vec::new();
    macro_rules! flushbuf {
        () => {
            if !buf.is_empty() {
                events.push(Event::Dialog(std::mem::take(&mut buf)));
            }
        };
    }
    for chk in raw.chunks(4) {
        for &b in chk.iter().rev() {
            match &mut status {
                Status::Init => match b {
                    0xD9 => {
                        flushbuf!();
                        events.push(Event::Sparkly);
                    }
                    0xFC => {
                        flushbuf!();
                        status = Status::Style;
                    }
                    0xF7 => {
                        flushbuf!();
                        events.push(Event::Space);
                    }
                    0xF0 => {
                        flushbuf!();
                        events.push(Event::Linebreak);
                    }
                    0xF1 => {
                        flushbuf!();
                        events.push(Event::Bell);
                    }
                    0xF2 => {
                        flushbuf!();
                        status = Status::Delay;
                    }
                    0xF3 => {
                        lookup_table = LookupTable::Kana;
                    }
                    0xF4 => {
                        lookup_table = LookupTable::Latin;
                    }
                    0xF5 => {
                        lookup_table = LookupTable::Kanji;
                    }
                    0xF6 => {
                        lookup_table = LookupTable::Button;
                    }
                    0xFB => {
                        flushbuf!();
                        events.push(Event::NextBubble);
                    }
                    0xFD => {
                        flushbuf!();
                        events.push(Event::End);
                    }
                    0xFF => {
                        flushbuf!();
                        status = Status::ExtCmd;
                    }
                    _ => 'nospecial: {
                        let (kind, opt_ch) = match lookup_table {
                            LookupTable::Kana => ("kana", charsets::kana(b)),
                            LookupTable::Kanji => ("kanji", charsets::kanji(b)),
                            LookupTable::Latin => ("latin", charsets::latin(b)),
                            LookupTable::Button => {
                                events.push(Event::ButtonRef {
                                    button: charsets::button(b),
                                    rawcode: b,
                                });
                                break 'nospecial;
                            }
                        };
                        match opt_ch {
                            Some(ch) => buf.push(ch),
                            None => buf.push_str(&format!("{{{kind}:{b:02X}}}")),
                        }
                    }
                },
                Status::Style => match Style::try_from(b) {
                    Ok(s) => {
                        events.push(Event::StyleChange(s));
                        status = Status::Init;
                    }
                    Err(e) => return Err(e.to_string()),
                },
                Status::Delay => {
                    events.push(Event::Delay(b));
                    status = Status::Init;
                }
                Status::ExtCmd => match extcmd::n_params(b) {
                    Some(argc) => {
                        if argc == 0 {
                            match extcmd::ExtCmd::from_id_and_args(b, &[]) {
                                Some(extcmd) => {
                                    events.push(Event::ExtCmd(extcmd));
                                }
                                None => events.push(Event::ExtCmdError {
                                    id: b,
                                    argc: 0,
                                    args_got: 0,
                                }),
                            }
                            status = Status::Init;
                        } else {
                            status = Status::ExtCmdParams {
                                argc,
                                argidx: 0,
                                id: b,
                            };
                            argbuf.clear();
                        }
                    }
                    None => {
                        events.push(Event::ExtCmd(extcmd::ExtCmd::Unknown(UnkCmd(b))));
                        status = Status::Init;
                    }
                },
                Status::ExtCmdParams { id, argc, argidx } => {
                    argbuf.push(b);
                    if (*argidx + 1) == *argc {
                        match extcmd::ExtCmd::from_id_and_args(*id, &argbuf) {
                            Some(extcmd) => {
                                events.push(Event::ExtCmd(extcmd));
                            }
                            None => events.push(Event::ExtCmdError {
                                id: *id,
                                argc: *argc,
                                args_got: argbuf.len() as u8,
                            }),
                        }
                        status = Status::Init;
                    } else {
                        *argidx += 1;
                    }
                }
            }
        }
    }
    flushbuf!();
    Ok(events)
}

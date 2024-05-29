use crate::{charsets::Button, LookupTable};

#[derive(Debug)]
#[allow(dead_code)]
pub enum Event {
    BubbleStyle(u8),
    Char(char),
    Btn(Button),
    UnkKana(u8),
    UnkKanji(u8),
    UnkLatin(u8),
    UnkBtn(u8),
    Newline,
    Space,
    NextBubble,
    UnkExtCmd(u8),
    ExtCmd1D(u8),
    ExtSetColor(u8),
    ExtStoreColor,
    ExtLoadColor,
    ExtCmd0B(u8),
    ExtCmd06(u8, u8),
    Tab,
    UnkTextEffect(u8),
    TextEffectStar(u8),
    TextEffectShaky1,
    TextEffectWavy1,
    TextEffectDarkStar,
    TextEffectNoise(u8),
    TextEffectShaky2(u8),
    TextEffectRainbow1,
    TextEffectWavy2,
    TextEffectRainbow2,
    TextEffectQuickPulse,
    TextEffectWavePulse,
    TextEffectShadow,
    /// Used when bowser laughs, and probably other places. Just makes the test go faster(?)
    TextEffectBowserLaugh,
    ExtCmd0C(u8),
    UnkExtExtCmd(u8),
    ExtExtVOffset(u8),
    ExtTextHoffset(u8),
    ExtCmdUnk14(u8),
    ExtCmdUnk15(u8),
}

type Iter<'a> = &'a mut (dyn Iterator<Item = u8> + 'a);

pub struct Decoder<'a> {
    events: Vec<Event>,
    lookup_table: LookupTable,
    iter: Iter<'a>,
}

impl<'a> Decoder<'a> {
    fn new(iter: Iter<'a>) -> Self {
        Self {
            events: Vec::new(),
            lookup_table: LookupTable::Kana,
            iter,
        }
    }
}

pub fn decode_events(raw: &[u8]) -> Vec<Event> {
    let mut iter = raw.chunks(4).flat_map(move |chk| chk.iter().rev().cloned());
    let mut decoder = Decoder::new(&mut iter);
    while decoder.next().is_some() {}
    decoder.events
}

impl<'a> Decoder<'a> {
    fn next(&mut self) -> Option<()> {
        match self.iter.next()? {
            0xF8 => {
                self.events.push(Event::BubbleStyle(self.iter.next()?));
            }
            0xF0 => self.events.push(Event::Newline),
            0xF1 => self.lookup_table = LookupTable::Kana,
            0xF2 => self.lookup_table = LookupTable::Latin,
            0xF3 => self.lookup_table = LookupTable::Kanji,
            0xF4 => self.lookup_table = LookupTable::Button,
            0xF5 => self.events.push(Event::Space),
            0xF6 => self.events.push(Event::Tab),
            0xFA => self.events.push(Event::NextBubble),
            0xFB => return None,
            0xFF => {
                let ev = self.next_extcmd()?;
                self.events.push(ev);
            }
            etc => self.add_char(etc),
        }
        Some(())
    }

    fn add_char(&mut self, byte: u8) {
        let ev = match self.lookup_table {
            LookupTable::Kana => match crate::charsets::kana(byte) {
                Some(ch) => Event::Char(ch),
                None => Event::UnkKana(byte),
            },
            LookupTable::Kanji => match crate::charsets::kanji(byte) {
                Some(ch) => Event::Char(ch),
                None => Event::UnkKanji(byte),
            },
            LookupTable::Latin => match crate::charsets::latin(byte) {
                Some(ch) => Event::Char(ch),
                None => Event::UnkLatin(byte),
            },
            LookupTable::Button => match crate::charsets::button(byte) {
                Some(b) => Event::Btn(b),
                None => Event::UnkBtn(byte),
            },
        };
        self.events.push(ev);
    }

    fn next_extcmd(&mut self) -> Option<Event> {
        let ev = match self.iter.next()? {
            0x04 => Event::ExtSetColor(self.iter.next()?),
            0x0B => Event::ExtCmd0B(self.iter.next()?),
            0x0C => Event::ExtCmd0C(self.iter.next()?),
            0x06 => Event::ExtCmd06(self.iter.next()?, self.iter.next()?),
            0x14 => Event::ExtCmdUnk14(self.iter.next()?),
            0x15 => Event::ExtCmdUnk15(self.iter.next()?),
            0x1C => self.next_text_effect()?,
            0x1D => Event::ExtCmd1D(self.iter.next()?),
            0x1A => Event::ExtStoreColor,
            0x1B => Event::ExtLoadColor,
            0x1E => Event::ExtTextHoffset(self.iter.next()?),
            0xFF => match self.iter.next()? {
                0x0B => Event::ExtExtVOffset(self.iter.next()?),
                etc => Event::UnkExtExtCmd(etc),
            },
            etc => Event::UnkExtCmd(etc),
        };
        Some(ev)
    }

    fn next_text_effect(&mut self) -> Option<Event> {
        let ev = match self.iter.next()? {
            0x00 => Event::TextEffectShaky1,
            0x01 => Event::TextEffectWavy1,
            0x02 => Event::TextEffectDarkStar,
            0x03 => Event::TextEffectNoise(self.iter.next()?),
            0x05 => Event::TextEffectShaky2(self.iter.next()?),
            0x06 => Event::TextEffectRainbow1,
            0x07 => Event::TextEffectStar(self.iter.next()?),
            0x08 => Event::TextEffectWavy2,
            0x09 => Event::TextEffectRainbow2,
            0x0A => Event::TextEffectBowserLaugh,
            0x0C => Event::TextEffectQuickPulse,
            0x0D => Event::TextEffectWavePulse,
            0x0E => Event::TextEffectShadow,
            etc => Event::UnkTextEffect(etc),
        };
        Some(ev)
    }
}

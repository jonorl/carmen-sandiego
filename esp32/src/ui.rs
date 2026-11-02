//! What the screens are made of: layout A (as the original, the clock and
//! picture on the left, the text panel and four buttons on the right), the
//! EGA colours, the double-line frames, the button icons, and text broken
//! into rows a panel can page through.

use crate::art::pixels::{Canvas, Rgb};
use crate::lang::tr;
use crate::rules::game::{Line, Tone};
use crate::text::{Family, Font};

pub const BLACK: Rgb = [0, 0, 0];
pub const WHITE: Rgb = [255, 255, 255];
pub const GREY: Rgb = [170, 170, 170];
pub const DARK: Rgb = [85, 85, 85];
pub const NAVY: Rgb = [0, 0, 170];
pub const BLUE: Rgb = [85, 85, 255];
pub const LIME: Rgb = [85, 255, 85];
pub const CYAN: Rgb = [85, 255, 255];
pub const RED: Rgb = [255, 85, 85];
pub const YELLOW: Rgb = [255, 255, 85];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

pub const fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect { x, y, w, h }
}

impl Rect {
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }

    /// Inside the frame's double line and gap.
    pub fn inner(&self) -> Rect {
        rect(self.x + INSET, self.y + INSET, self.w - 2 * INSET, self.h - 2 * INSET)
    }
}

pub const CLOCK: Rect = rect(0, 0, 150, 34);
pub const PICTURE: Rect = rect(0, 36, 150, 204);
pub const TEXT: Rect = rect(152, 0, 168, 186);
pub const BUTTONS: [Rect; 4] = [rect(152, 188, 42, 52), rect(194, 188, 42, 52), rect(236, 188, 42, 52), rect(278, 188, 42, 52)];

/// The four buttons, in order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Button {
    Notes,
    Depart,
    Investigate,
    Crime,
}

pub const BUTTON_ORDER: [Button; 4] = [Button::Notes, Button::Depart, Button::Investigate, Button::Crime];

/// The frames' double line and gap take this much off each side.
const INSET: i32 = 4;
/// Space between a panel's edge and its text.
pub const PAD: i32 = 3;

pub struct Fonts {
    /// Pixelify Sans, 13 px smoothed: everything the panels say.
    pub body: Font,
    /// Its own pixels at 11 px: button labels.
    pub label: Font,
    /// Doubled, 22 px: headings.
    pub heading: Font,
}

impl Fonts {
    pub fn new() -> Fonts {
        let font = |family, size| Font::new(family, size).expect("a size built by build.rs");
        Fonts { body: font(Family::PixelifySmooth, 13), label: font(Family::Pixelify, 11), heading: font(Family::Pixelify, 22) }
    }
}

impl Default for Fonts {
    fn default() -> Self {
        Fonts::new()
    }
}

/// The original's frames: a double white line.
pub fn frame(cv: &mut Canvas, b: Rect, colour: Rgb) {
    for k in [0, 2] {
        let (x, y, w, h) = (b.x + k, b.y + k, b.w - 2 * k, b.h - 2 * k);
        cv.rect(x, y, w, 1, colour);
        cv.rect(x, y + h - 1, w, 1, colour);
        cv.rect(x, y, 1, h, colour);
        cv.rect(x + w - 1, y, 1, h, colour);
    }
}

/// A button: its frame (yellow while pressed), and its icon, dimmed when off.
pub fn button(cv: &mut Canvas, f: &Fonts, which: Button, at: Rect, pressed: bool, on: bool) {
    frame(cv, at, if pressed { YELLOW } else { WHITE });
    let b = at.inner();
    if pressed {
        cv.rect(b.x, b.y, b.w, b.h, NAVY);
    }
    let ink = if on { WHITE } else { DARK };
    let cx = b.x + b.w / 2;
    let label = match which {
        Button::Notes => tr("NOTES", "NOTAS"),
        Button::Depart => tr("DEPART", "VIAJAR"),
        Button::Investigate => "",
        Button::Crime => tr("CRIME", "CRIMEN"),
    };
    if !label.is_empty() {
        f.label.draw(cv, cx - f.label.width(label) / 2, b.y + f.label.ascent(), label, ink);
    }
    let (top, bottom) = (b.y + f.label.line_height(), b.y + b.h);
    let cy = (top + bottom) / 2;
    let back = if pressed { NAVY } else { BLACK };
    // Carmen's icons, narrowed to half its buttons' width and turned light on dark.
    match which {
        Button::Notes => {
            // A spiral-bound notepad.
            let (x, y, w, h) = (cx - 11, top + 5, 22, bottom - top - 8);
            cv.rect(x, y, w, h, ink);
            for ly in (y + 6..y + h - 2).step_by(4) {
                cv.rect(x + 3, ly, w - 6, 1, if on { BLUE } else { GREY });
            }
            for i in 0..5 {
                cv.rect(x + 2 + i * 4, y - 2, 2, 4, GREY);
            }
        }
        Button::Depart => {
            // A plane seen head-on: wings, tail fin, fuselage and engines.
            let y = cy + 4;
            let body = if on { GREY } else { DARK };
            cv.rect(cx - 15, y, 30, 2, ink);
            cv.rect(cx - 1, y - 10, 2, 9, ink);
            cv.rect(cx - 5, y - 7, 10, 1, ink);
            cv.disc(cx, y + 1, 3, body);
            cv.disc(cx - 8, y + 4, 2, body);
            cv.disc(cx + 8, y + 4, 2, body);
        }
        Button::Investigate => {
            // A magnifying glass with a question mark in the lens.
            let (lx, ly) = (cx + 3, b.y + b.h / 2 - 4);
            for d in -1..=1 {
                cv.line(lx - 7 + d, ly + 7, lx - 15 + d, ly + 16, ink);
                cv.line(lx - 7, ly + 7 + d, lx - 15, ly + 16 + d, ink);
            }
            cv.disc(lx, ly, 11, ink);
            cv.disc(lx, ly, 8, if on { [207, 227, 255] } else { back });
            let q = "?";
            f.heading.draw(cv, lx - f.heading.width(q) / 2, ly + f.heading.ascent() / 2 - 1, q, if on { NAVY } else { DARK });
        }
        Button::Crime => {
            // A computer: a monitor with lines of text, and a keyboard.
            let (x, y, w, h) = (cx - 14, top + 3, 28, bottom - top - 13);
            cv.rect(x, y, w, h, if on { GREY } else { DARK });
            cv.rect(x + 2, y + 2, w - 4, h - 4, if on { BLUE } else { back });
            for ly in (y + 5..y + h - 4).step_by(3) {
                cv.rect(x + 5, ly, w - 10 - (ly - y) % 4 * 2, 1, ink);
            }
            cv.rect(cx - 16, bottom - 8, 32, 5, ink);
            for i in 0..7 {
                cv.rect(cx - 14 + i * 4, bottom - 6, 2, 1, if on { DARK } else { BLACK });
            }
        }
    }
}

/// One line of a panel's text, already wrapped.
#[derive(Clone, Debug)]
pub struct Row {
    pub text: String,
    pub colour: Rgb,
    /// How many of its characters are a speaker's name, drawn in cyan.
    pub speaker: usize,
}

impl Row {
    pub fn gap() -> Row {
        Row { text: String::new(), colour: WHITE, speaker: 0 }
    }
}

pub fn tone_colour(tone: Tone) -> Rgb {
    match tone {
        Tone::Plain => WHITE,
        Tone::Dim => GREY,
        Tone::Heading => YELLOW,
        Tone::Good => LIME,
        Tone::Warn => YELLOW,
        Tone::Bad => RED,
    }
}

/// `text` wrapped to `width` in `colour`.
pub fn rows(f: &Font, text: &str, colour: Rgb, width: i32) -> Vec<Row> {
    f.wrap(text, width).into_iter().map(|text| Row { text, colour, speaker: 0 }).collect()
}

/// The game's lines wrapped to `width`: a witness's name before what they say,
/// and an empty line as a small gap.
pub fn log_rows(f: &Font, lines: &[Line], width: i32) -> Vec<Row> {
    let mut out = Vec::new();
    for line in lines {
        if line.text.is_empty() && line.speaker.is_none() {
            out.push(Row::gap());
            continue;
        }
        let colour = tone_colour(line.tone);
        match &line.speaker {
            Some(who) => {
                let said = format!("{who}: {}", line.text);
                let mut wrapped = rows(f, &said, colour, width);
                if let Some(first) = wrapped.first_mut() {
                    first.speaker = (who.chars().count() + 1).min(first.text.chars().count());
                }
                out.extend(wrapped);
            }
            None => out.extend(rows(f, &line.text, colour, width)),
        }
    }
    out
}

/// Rows shown a page at a time; a tap on the panel turns the page.
#[derive(Clone, Debug, Default)]
pub struct Pager {
    pub rows: Vec<Row>,
    pub page: usize,
}

impl Pager {
    pub fn new(rows: Vec<Row>) -> Pager {
        // Gaps at the top of a page would only waste it.
        let mut rows = rows;
        while rows.first().is_some_and(|r| r.text.is_empty()) {
            rows.remove(0);
        }
        Pager { rows, page: 0 }
    }

    fn per_page(f: &Font, area: Rect) -> usize {
        ((area.h / f.line_height()).max(1)) as usize
    }

    pub fn pages(&self, f: &Font, area: Rect) -> usize {
        self.rows.len().div_ceil(Pager::per_page(f, area)).max(1)
    }

    pub fn last_page(&self, f: &Font, area: Rect) -> bool {
        self.page + 1 >= self.pages(f, area)
    }

    /// Shows the next page; false if this was the last.
    pub fn turn(&mut self, f: &Font, area: Rect) -> bool {
        if self.last_page(f, area) {
            return false;
        }
        self.page += 1;
        true
    }

    /// Draws this page in `area`, with a ▼ at the bottom right if more follows.
    pub fn draw(&self, cv: &mut Canvas, f: &Font, area: Rect) {
        let n = Pager::per_page(f, area);
        for (k, row) in self.rows.iter().skip(self.page * n).take(n).enumerate() {
            let y = area.y + k as i32 * f.line_height() + f.ascent();
            let split = row.text.char_indices().nth(row.speaker).map_or(row.text.len(), |(i, _)| i);
            let x = f.draw(cv, area.x, y, &row.text[..split], CYAN);
            f.draw(cv, x, y, &row.text[split..], row.colour);
        }
        if !self.last_page(f, area) {
            more(cv, area);
        }
    }
}

/// The ▼ that says a tap shows more.
pub fn more(cv: &mut Canvas, area: Rect) {
    let (x, y) = (area.x + area.w - 8, area.y + area.h - 5);
    cv.poly(&[(x, y), (x + 7, y), (x + 3, y + 4)], YELLOW);
}

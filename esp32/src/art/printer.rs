//! The printer, after Carmen's `screens/printer.rs`: text queued with
//! `print` comes out a line at a time, quickly, onto tractor-feed paper,
//! which feeds up as lines come. Drawn into the picture frame, under the crime computer
//! for warrants, or filling it for the promotion at the end of a case.

use std::collections::VecDeque;
use std::time::Instant;

use crate::art::pixels::{Canvas, Rgb};
use crate::text::Font;
use crate::ui::Rect;

/// Lines printed per second.
const SPEED: f64 = 8.0;
/// The printer's own height, under the paper.
const CASE_H: i32 = 22;
/// The paper's edge strips with the tractor holes, either side of the text.
const MARGIN: i32 = 15;
/// Pixels between tractor holes.
const HOLES: i32 = 8;

const PAPER: Rgb = [255, 255, 255];
const INK: Rgb = [0, 0, 0];
const HOLE: Rgb = [0x22, 0x22, 0x22];
const RULE: Rgb = [0xbb, 0xbb, 0xbb];
const BEIGE: Rgb = [0xe3, 0xd3, 0xb8];
const BEIGE_LIGHT: Rgb = [0xf4, 0xec, 0xdc];
const SHADE: Rgb = [0xcb, 0xb8, 0x9a];
const BASE: Rgb = [0x6f, 0x8d, 0xb3];
const BASE_LIGHT: Rgb = [0x8a, 0xa6, 0xc8];

pub struct Printer {
    /// Lines on the paper, newest last.
    lines: Vec<String>,
    pending: VecDeque<String>,
    /// Lines due but not yet printed, and since when they're counted.
    owed: f64,
    since: Instant,
}

impl Default for Printer {
    fn default() -> Self {
        Printer::new()
    }
}

impl Printer {
    pub fn new() -> Printer {
        Printer { lines: Vec::new(), pending: VecDeque::new(), owed: 0.0, since: Instant::now() }
    }

    /// How wide a line of text can be on paper in a frame `w` pixels wide.
    pub fn text_width(w: i32) -> i32 {
        w - 2 * MARGIN
    }

    /// Queues `text` from a fresh line, wrapped to `width` in `f`; '\n' starts a new line.
    pub fn print(&mut self, f: &Font, text: &str, width: i32) {
        if !self.busy() {
            // The first line comes out at once.
            self.since = Instant::now();
            self.owed = 1.0;
        }
        for para in text.split('\n') {
            let lines = f.wrap(para, width);
            if lines.is_empty() {
                self.pending.push_back(String::new());
            }
            self.pending.extend(lines);
        }
    }

    pub fn busy(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Prints the rest at once (the player tapped).
    pub fn finish(&mut self) {
        self.lines.extend(self.pending.drain(..));
    }

    /// Prints what's come due since the last call; returns how many
    /// lines came out, for the sound.
    pub fn update(&mut self) -> usize {
        let now = Instant::now();
        self.owed += now.duration_since(self.since).as_secs_f64() * SPEED;
        self.since = now;
        let mut printed = 0;
        while self.owed >= 1.0 {
            let Some(line) = self.pending.pop_front() else { break };
            self.owed -= 1.0;
            self.lines.push(line);
            printed += 1;
        }
        if self.pending.is_empty() {
            self.owed = 0.0;
        }
        printed
    }

    /// Draws the printer filling `r`: the case at the bottom, and as much
    /// paper above it as has been printed on, newest line at the bottom.
    /// `echo` is a line being typed, shown after the printed ones.
    pub fn draw(&self, cv: &mut Canvas, f: &Font, r: Rect, echo: Option<&str>) {
        let lines: Vec<&str> = self.lines.iter().map(String::as_str).chain(echo).collect();
        let lh = f.line_height();
        let slot = r.y + r.h - CASE_H;
        let top = (slot - (lines.len() as i32 * lh + 6)).max(r.y);
        let (px, pw) = (r.x + 6, r.w - 12);
        cv.rect(px, top, pw, slot - top, PAPER);
        cv.rect(px + 9, top, 1, slot - top, RULE);
        cv.rect(px + pw - 10, top, 1, slot - top, RULE);
        // Tractor-feed holes down both edges, moving up a line at a time.
        let fed = lines.len() as i32 * lh;
        let mut y = slot - 4 - (HOLES - fed.rem_euclid(HOLES)) % HOLES;
        while y >= top {
            cv.rect(px + 3, y, 3, 2, HOLE);
            cv.rect(px + pw - 6, y, 3, 2, HOLE);
            y -= HOLES;
        }

        // Lines, newest just above the slot.
        let text_x = r.x + MARGIN;
        let base = slot - 4;
        let count = lines.len() as i32;
        for (i, line) in lines.iter().enumerate() {
            let y = base - (count - 1 - i as i32) * lh;
            if y - f.ascent() < top {
                continue;
            }
            f.draw(cv, text_x, y, line, INK);
        }

        // The printer: a beige case with the paper's slot and buttons, on a blue base.
        let case = Rect { x: r.x + 1, y: slot, w: r.w - 2, h: 14 };
        cv.rect(case.x, case.y, case.w, case.h, BEIGE);
        cv.rect(case.x, case.y + 1, case.w, 2, BEIGE_LIGHT);
        cv.rect(case.x, case.y + 10, case.w, 4, SHADE);
        cv.rect(px + 2, slot, pw - 4, 2, INK);
        for (n, x) in [case.w - 34, case.w - 24, case.w - 14].into_iter().enumerate() {
            cv.rect(case.x + x, case.y + 5, 7, 3, if n == 0 { [0xf0, 0xa0, 0x40] } else { [0xd8, 0x8a, 0x30] });
        }
        cv.rect(r.x, slot + 14, r.w, CASE_H - 14, BASE);
        cv.rect(r.x, slot + 14, r.w, 2, BASE_LIGHT);
    }
}

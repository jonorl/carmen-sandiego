//! The crime computer as Carmen's `screens/computer.rs` draws it (a beige
//! CRT on a stand, on a desktop case with two drive bays), shrunk to the top
//! of the picture frame, with the printer under it.

use crate::art::pixels::{Canvas, Rgb};
use crate::ui::{Rect, rect};

const BEIGE: Rgb = [0xdf, 0xd2, 0xbc];
const BEIGE_LIGHT: Rgb = [0xf2, 0xea, 0xdc];
const BEIGE_DARK: Rgb = [0xb8, 0xa8, 0x8e];
const BEIGE_SHADE: Rgb = [0xcb, 0xbc, 0xa2];
const BEZEL_INNER: Rgb = [0x4a, 0x4a, 0x4a];
const SLOT: Rgb = [0x2a, 0x26, 0x22];
const LIGHTS_OFF: Rgb = [0x3c, 0x5a, 0xa8];

/// How tall the computer is drawn.
pub const H: i32 = 84;

/// The 16 EGA colours the lights flash through while computing.
const EGA: [u32; 16] = [
    0x000000, 0x0000aa, 0x00aa00, 0x00aaaa, 0xaa0000, 0xaa00aa, 0xaa5500, 0xaaaaaa, 0x555555, 0x5555ff, 0x55ff55, 0x55ffff, 0xff5555,
    0xff55ff, 0xffff55, 0xffffff,
];

/// Light `i`'s colour `t` seconds into computing. Each light changes at its
/// own pace, to a colour picked by scrambling the light and the moment.
fn flashing(i: usize, t: f64) -> Rgb {
    let rate = [7.0, 9.5, 5.5][i % 3];
    let n = (t * rate + i as f64 * 0.37) as u64;
    // SplitMix64's finaliser: every bit of the input stirs every bit out.
    let mut z = (n * 3 + i as u64).wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    let c = EGA[(z % 16) as usize];
    [(c >> 16) as u8, (c >> 8) as u8, c as u8]
}

/// A rectangle with its corners cut, for the CRT's rounded bezel.
fn rounded(cv: &mut Canvas, r: Rect, c: Rgb) {
    cv.rect(r.x + 1, r.y, r.w - 2, r.h, c);
    cv.rect(r.x, r.y + 1, r.w, r.h - 2, c);
}

/// Draws the computer `H` pixels tall across the top of `r` and returns its
/// screen, for the caller to write on. `computing` is how long it has been
/// working on a warrant, if it is: the lights on its stand flash meanwhile.
pub fn draw(cv: &mut Canvas, r: Rect, computing: Option<f64>) -> Rect {
    let at = |x: i32, y: i32, w: i32, h: i32| rect(r.x + x, r.y + y, w, h);
    let cx = r.w / 2;

    // The monitor: a rounded beige bezel around a recessed, dark screen.
    let bw = r.w - 8;
    rounded(cv, at(cx - bw / 2, 0, bw, 60), BEIGE);
    cv.rect(r.x + cx - bw / 2 + 3, r.y + 1, bw - 6, 1, BEIGE_LIGHT);
    rounded(cv, at(cx - bw / 2 + 4, 4, bw - 8, 52), BEIGE_DARK);
    rounded(cv, at(cx - bw / 2 + 6, 6, bw - 12, 48), BEZEL_INNER);
    let screen = at(cx - bw / 2 + 8, 8, bw - 16, 44);
    cv.rect(screen.x, screen.y, screen.w, screen.h, [0, 0, 0]);

    // The stand, with vents either side of a panel of lights.
    let stand = at(cx - 37, 60, 74, 8);
    cv.rect(stand.x, stand.y, stand.w, stand.h, BEIGE_DARK);
    for i in 0..3 {
        cv.rect(stand.x + 4 + i * 4, stand.y + 2, 2, 4, SLOT);
        cv.rect(stand.x + stand.w - 6 - i * 4, stand.y + 2, 2, 4, SLOT);
    }
    cv.rect(r.x + cx - 19, r.y + 61, 38, 6, [0x1a, 0x1a, 0x2a]);
    for i in 0..3 {
        let colour = computing.map_or(LIGHTS_OFF, |t| flashing(i, t));
        cv.rect(r.x + cx - 16 + i as i32 * 11, r.y + 63, 9, 2, colour);
    }

    // The desktop case with two drive bays and a label plate.
    let case = at(cx - (bw - 12) / 2, 68, bw - 12, 16);
    cv.rect(case.x, case.y, case.w, case.h, BEIGE);
    cv.rect(case.x, case.y, case.w, 1, BEIGE_LIGHT);
    cv.rect(case.x + 10, case.y + 6, 30, 5, BEIGE_DARK);
    for y in [2, 9] {
        let bay = rect(case.x + case.w - 54, case.y + y, 48, 6);
        cv.rect(bay.x, bay.y, bay.w, bay.h, BEIGE_SHADE);
        cv.rect(bay.x + 4, bay.y + 2, bay.w - 8, 2, SLOT);
    }
    screen
}

//! The map shown when departing: Natural Earth land outlines, rasterised
//! into a canvas for whatever area the view covers, as Carmen's `art/map.rs`
//! does into a texture.

use crate::art::pixels::{Canvas, Rgb};

/// Land polygons as rings of (lon, lat) points.
pub struct Land {
    rings: Vec<Vec<(f32, f32)>>,
}

impl Land {
    /// Reads the outlines `build.rs` packed from Natural Earth's GeoJSON.
    pub fn load() -> Land {
        let bytes: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/land.bin"));
        let word = |at: usize| [bytes[at], bytes[at + 1]];
        let mut rings = Vec::new();
        let mut at = 0;
        while at + 2 <= bytes.len() {
            let n = u16::from_le_bytes(word(at)) as usize;
            at += 2;
            let degrees = |at: usize| i16::from_le_bytes(word(at)) as f32 / 100.0;
            rings.push((0..n).map(|k| (degrees(at + 4 * k), degrees(at + 4 * k + 2))).collect());
            at += 4 * n;
        }
        Land { rings }
    }
}

const OCEAN: Rgb = [0x00, 0x00, 0xaa];
const GRID: Rgb = [0x22, 0x22, 0xcc];
const LAND: Rgb = [0x00, 0xaa, 0x00];
const COAST: Rgb = [0x55, 0xff, 0x55];

/// An equirectangular view `w` by `h` pixels: longitude is squeezed by
/// cos(latitude) at the view's centre so shapes look about right at any zoom.
#[derive(Clone, Copy)]
pub struct MapView {
    w: i32,
    h: i32,
    lon0: f32,
    lat0: f32,
    kx: f32,
    /// Pixels per degree of latitude.
    ppd: f32,
}

impl MapView {
    /// A view that fits all the given (lat, lon) points with a margin.
    pub fn fit(w: i32, h: i32, points: &[(f64, f64)]) -> MapView {
        let mean = points.iter().map(|p| p.0 as f32).sum::<f32>() / points.len().max(1) as f32;
        let kx = mean.clamp(-60.0, 60.0).to_radians().cos();
        let xs = points.iter().map(|p| p.1 as f32 * kx);
        let ys = points.iter().map(|p| p.0 as f32);
        let (x0, x1) = xs.fold((f32::MAX, f32::MIN), |(a, b), v| (a.min(v), b.max(v)));
        let (y0, y1) = ys.fold((f32::MAX, f32::MIN), |(a, b), v| (a.min(v), b.max(v)));
        let span_x = (x1 - x0).max(30.0) * 1.35;
        let span_y = (y1 - y0).max(30.0) * 1.35;
        // Never zoom out further than the whole world fits across.
        let ppd = (w as f32 / span_x).min(h as f32 / span_y).max(w as f32 / (360.0 * kx));
        MapView { w, h, lon0: (x0 + x1) / 2.0 / kx, lat0: (y0 + y1) / 2.0, kx, ppd }
    }

    /// Where (lat, lon) falls, in pixels from the view's top left.
    pub fn project(&self, lat: f64, lon: f64) -> (f32, f32) {
        (self.w as f32 / 2.0 + (lon as f32 - self.lon0) * self.kx * self.ppd, self.h as f32 / 2.0 - (lat as f32 - self.lat0) * self.ppd)
    }

    /// Paints the land by scanline filling the polygons (even-odd rule),
    /// then marks the coast and a dotted grid.
    pub fn render(&self, land: &Land) -> Canvas {
        let (w, h) = (self.w as usize, self.h as usize);
        let mut edges = Vec::new();
        for ring in &land.rings {
            let pts: Vec<(f32, f32)> = ring.iter().map(|&(lon, lat)| self.project(lat as f64, lon as f64)).collect();
            edges.extend(pts.windows(2).map(|p| (p[0], p[1])));
        }
        let mut is_land = vec![false; w * h];
        let mut crossings = Vec::new();
        for py in 0..h {
            let yc = py as f32 + 0.5;
            crossings.clear();
            for &(a, b) in &edges {
                if (a.1 <= yc) != (b.1 <= yc) {
                    crossings.push(a.0 + (yc - a.1) * (b.0 - a.0) / (b.1 - a.1));
                }
            }
            crossings.sort_by(f32::total_cmp);
            for pair in crossings.as_chunks::<2>().0 {
                let from = (pair[0] - 0.5).ceil().max(0.0) as usize;
                let to = ((pair[1] - 0.5).ceil().max(0.0) as usize).min(w);
                for px in from..to {
                    is_land[py * w + px] = true;
                }
            }
        }

        // Dotted lines every 30 degrees of latitude and longitude.
        let step = 30.0;
        let lon_at = |px: usize| self.lon0 + (px as f32 + 0.5 - w as f32 / 2.0) / (self.kx * self.ppd);
        let lat_at = |py: usize| self.lat0 - (py as f32 + 0.5 - h as f32 / 2.0) / self.ppd;
        let near_line = |deg: f32, per_px: f32| (deg / step - (deg / step).round()).abs() * step < per_px / 2.0;
        let (lon_px, lat_px) = (1.0 / (self.kx * self.ppd), 1.0 / self.ppd);

        let mut cv = Canvas::new(self.w, self.h);
        for py in 0..h {
            for px in 0..w {
                let colour = if is_land[py * w + px] {
                    let neighbours = [(px.wrapping_sub(1), py), (px + 1, py), (px, py.wrapping_sub(1)), (px, py + 1)];
                    let coast = neighbours.iter().any(|&(x, y)| x < w && y < h && !is_land[y * w + x]);
                    if coast { COAST } else { LAND }
                } else if (px + py) % 2 == 0 && (near_line(lon_at(px), lon_px) || near_line(lat_at(py), lat_px)) {
                    GRID
                } else {
                    OCEAN
                };
                cv.set(px as i32, py as i32, colour);
            }
        }
        cv
    }
}

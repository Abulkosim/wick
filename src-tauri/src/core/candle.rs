/// Wick's mark: a candle that burns down as a timed session runs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Glyph {
    /// Unlit candle.
    Off,
    /// Lit, with the body filled to the share of time left.
    Remaining(f64),
    /// Lit, full: awake until stopped.
    Full,
    /// Lit with a hollow body: an app or command holds the Mac awake.
    Held,
}

#[derive(Clone, Copy, PartialEq)]
enum Part {
    Solid,
    Track,
}

const SAMPLES: usize = 8;
const LEFT: f64 = 0.30;
const RIGHT: f64 = 0.70;
const TOP: f64 = 0.46;
const BOTTOM: f64 = 0.96;
const CORNER: f64 = 0.07;
const STROKE: f64 = 0.085;
const FLAME_RADIUS: f64 = 0.115;
const FLAME_HEIGHT: f64 = 0.36;
const FLAME_GAP: f64 = 0.045;
const WICK: f64 = 0.04;
const TRACK_ALPHA: f64 = 0.5;

/// Inside a rounded rectangle, in unit coordinates.
fn in_body(x: f64, y: f64, top: f64, inset: f64) -> bool {
    let (left, right, bottom) = (LEFT + inset, RIGHT - inset, BOTTOM - inset);
    let top = top + inset;
    if x < left || x > right || y < top || y > bottom {
        return false;
    }
    let corner = (CORNER - inset)
        .max(0.0)
        .min((bottom - top) / 2.0)
        .min((right - left) / 2.0);
    let cx = x.clamp(left + corner, right - corner);
    let cy = y.clamp(top + corner, bottom - corner);
    (x - cx).hypot(y - cy) <= corner
}

/// A teardrop: a circle with a cone rising to a point.
fn in_flame(x: f64, y: f64, base: f64) -> bool {
    let cy = base - FLAME_RADIUS;
    let tip = base - FLAME_HEIGHT;
    if (x - 0.5).hypot(y - cy) <= FLAME_RADIUS {
        return true;
    }
    y >= tip && y <= cy && (x - 0.5).abs() <= FLAME_RADIUS * (y - tip) / (cy - tip)
}

fn part_at(x: f64, y: f64, glyph: Glyph) -> Option<Part> {
    let level = match glyph {
        Glyph::Off => 0.0,
        Glyph::Remaining(value) => value.clamp(0.0, 1.0),
        Glyph::Full | Glyph::Held => 1.0,
    };
    let lit = !matches!(glyph, Glyph::Off) && level > 0.0;
    let height = BOTTOM - TOP;
    let surface = BOTTOM - height * level.max(0.12);
    if lit && in_flame(x, y, surface - FLAME_GAP) {
        return Some(Part::Solid);
    }
    let outline = in_body(x, y, TOP, 0.0) && !in_body(x, y, TOP, STROKE);
    match glyph {
        Glyph::Held => outline.then_some(Part::Solid),
        _ if lit => in_body(x, y, surface, 0.0).then_some(Part::Solid),
        _ => {
            let stub = (x - 0.5).abs() <= WICK / 2.0 && (TOP - 0.1..TOP + STROKE).contains(&y);
            (outline || stub).then_some(Part::Track)
        }
    }
}

pub struct Pixel {
    pub coverage: f64,
    pub alpha: u8,
}

pub fn raster(size: u32, glyph: Glyph) -> Vec<Pixel> {
    let extent = f64::from(size);
    let total = (SAMPLES * SAMPLES) as f64;
    let mut pixels = Vec::with_capacity((size * size) as usize);
    for y in 0..size {
        for x in 0..size {
            let (mut covered, mut alpha) = (0.0, 0.0);
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let ux = (f64::from(x) + (sx as f64 + 0.5) / SAMPLES as f64) / extent;
                    let uy = (f64::from(y) + (sy as f64 + 0.5) / SAMPLES as f64) / extent;
                    if let Some(part) = part_at(ux, uy, glyph) {
                        covered += 1.0;
                        alpha += if part == Part::Track {
                            TRACK_ALPHA
                        } else {
                            1.0
                        };
                    }
                }
            }
            pixels.push(Pixel {
                coverage: covered / total,
                alpha: (alpha / total * 255.0).round() as u8,
            });
        }
    }
    pixels
}

/// Black template pixels; macOS tints them for the menu bar.
pub fn template_rgba(size: u32, glyph: Glyph) -> Vec<u8> {
    raster(size, glyph)
        .into_iter()
        .flat_map(|pixel| [0, 0, 0, pixel.alpha])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZES: [u32; 3] = [16, 22, 32];
    const GLYPHS: [(&str, Glyph); 6] = [
        ("off", Glyph::Off),
        ("full", Glyph::Full),
        ("75", Glyph::Remaining(0.75)),
        ("30", Glyph::Remaining(0.3)),
        ("5", Glyph::Remaining(0.05)),
        ("held", Glyph::Held),
    ];
    const BARS: [(&str, [u8; 3], u8); 2] = [("light", [0xf2; 3], 0), ("dark", [0x26; 3], 0xff)];

    fn luminance(value: f64) -> f64 {
        let value = value / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }

    fn contrast(a: f64, b: f64) -> f64 {
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    fn composite(alpha: u8, tint: u8, background: u8) -> f64 {
        let alpha = f64::from(alpha) / 255.0;
        f64::from(tint) * alpha + f64::from(background) * (1.0 - alpha)
    }

    fn at(size: u32, glyph: Glyph, ux: f64, uy: f64) -> u8 {
        let x = (ux * f64::from(size)) as u32;
        let y = (uy * f64::from(size)) as u32;
        raster(size, glyph)[(y * size + x) as usize].alpha
    }

    #[test]
    fn every_glyph_reads_on_light_and_dark_menu_bars() {
        for (bar, background, tint) in BARS {
            for size in SIZES {
                for (name, glyph) in GLYPHS {
                    let min = raster(size, glyph)
                        .iter()
                        .filter(|pixel| pixel.coverage >= 1.0)
                        .map(|pixel| {
                            contrast(
                                composite(pixel.alpha, tint, background[0]),
                                f64::from(background[0]),
                            )
                        })
                        .fold(f64::INFINITY, f64::min);
                    assert!(min.is_finite(), "{bar} {size} {name} has no solid pixels");
                    assert!(min >= 3.0, "{bar} {size} {name} contrast {min:.2}");
                }
            }
        }
    }

    #[test]
    fn lit_unlit_burning_and_held_differ_where_they_should() {
        for size in [22, 32] {
            let middle = (0.5, 0.8);
            assert_eq!(at(size, Glyph::Full, middle.0, middle.1), 255);
            assert_eq!(at(size, Glyph::Held, middle.0, middle.1), 0);
            assert_eq!(at(size, Glyph::Off, middle.0, middle.1), 0);
            let flame = (0.5, TOP - FLAME_GAP - FLAME_RADIUS);
            assert_eq!(at(size, Glyph::Full, flame.0, flame.1), 255);
            assert_eq!(at(size, Glyph::Held, flame.0, flame.1), 255);
            assert!(at(size, Glyph::Off, flame.0, flame.1) < 255);
            let upper_body = (LEFT + 0.06, TOP + 0.05);
            assert_eq!(
                at(size, Glyph::Remaining(0.3), upper_body.0, upper_body.1),
                0
            );
            assert_eq!(at(size, Glyph::Full, upper_body.0, upper_body.1), 255);
        }
        for size in SIZES {
            let rendered: Vec<Vec<u8>> = GLYPHS
                .iter()
                .map(|(_, glyph)| template_rgba(size, *glyph))
                .collect();
            for (i, a) in rendered.iter().enumerate() {
                for b in &rendered[i + 1..] {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn remaining_clamps_to_the_ends() {
        assert_eq!(
            template_rgba(32, Glyph::Remaining(1.4)),
            template_rgba(32, Glyph::Full)
        );
        assert_eq!(
            template_rgba(32, Glyph::Remaining(-1.0)),
            template_rgba(32, Glyph::Off)
        );
    }

    #[test]
    #[ignore = "writes docs/images/tray previews"]
    fn write_previews() {
        let directory =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/images/tray");
        std::fs::create_dir_all(&directory).unwrap();
        let cell = 40_u32;
        let width = cell * GLYPHS.len() as u32;
        let height = cell * (SIZES.len() * BARS.len()) as u32;
        let mut sheet = vec![0_u8; (width * height * 3) as usize];
        for (band, (_, background, tint)) in BARS.into_iter().enumerate() {
            for (row, size) in SIZES.into_iter().enumerate() {
                for (column, (_, glyph)) in GLYPHS.into_iter().enumerate() {
                    let pixels = raster(size, glyph);
                    let top = (band * SIZES.len() + row) as u32 * cell;
                    let left = column as u32 * cell;
                    let inset = (cell - size) / 2;
                    for y in 0..cell {
                        for x in 0..cell {
                            let mut value = f64::from(background[0]);
                            if (inset..inset + size).contains(&x)
                                && (inset..inset + size).contains(&y)
                            {
                                let pixel = &pixels[((y - inset) * size + x - inset) as usize];
                                value = composite(pixel.alpha, tint, background[0]);
                            }
                            let offset = (((top + y) * width + left + x) * 3) as usize;
                            sheet[offset..offset + 3].fill(value.round() as u8);
                        }
                    }
                }
            }
        }
        let zoom = 4;
        let mut zoomed = vec![0_u8; (width * height * 3 * zoom * zoom) as usize];
        for y in 0..height * zoom {
            for x in 0..width * zoom {
                let source = (((y / zoom) * width + x / zoom) * 3) as usize;
                let target = ((y * width * zoom + x) * 3) as usize;
                zoomed[target..target + 3].copy_from_slice(&sheet[source..source + 3]);
            }
        }
        for (file, data, scale) in [
            ("contact-sheet.png", &sheet, 1),
            ("contact-sheet@4x.png", &zoomed, zoom),
        ] {
            let output = std::fs::File::create(directory.join(file)).unwrap();
            let mut encoder = png::Encoder::new(output, width * scale, height * scale);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(data)
                .unwrap();
        }
    }
}

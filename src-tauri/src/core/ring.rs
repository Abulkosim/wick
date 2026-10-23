use std::f64::consts::{FRAC_PI_2, TAU};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Glyph {
    Off,
    Remaining(f64),
    Full,
    Held,
}

#[derive(Clone, Copy, PartialEq)]
enum Part {
    Arc,
    Track,
    Dot,
}

const SAMPLES: usize = 8;
const OUTER: f64 = 0.47;
const STROKE: f64 = 0.17;
const TRACK: f64 = 0.55;
const DOT: f64 = 0.14;
const TRACK_ALPHA: f64 = 0.5;

fn part_at(dx: f64, dy: f64, size: f64, glyph: Glyph) -> Option<Part> {
    let stroke = STROKE * size;
    let mid = OUTER * size - stroke / 2.0;
    let radius = dx.hypot(dy);
    if glyph == Glyph::Held && radius <= DOT * size {
        return Some(Part::Dot);
    }
    let in_band = (radius - mid).abs() <= stroke / 2.0;
    let value = match glyph {
        Glyph::Off => 0.0,
        Glyph::Remaining(value) => value.clamp(0.0, 1.0),
        Glyph::Full | Glyph::Held => 1.0,
    };
    if value >= 1.0 && in_band {
        return Some(Part::Arc);
    }
    if value > 0.0 {
        let angle = (dy.atan2(dx) + FRAC_PI_2).rem_euclid(TAU) / TAU;
        let cap = |turn: f64| {
            let theta = turn * TAU - FRAC_PI_2;
            (dx - mid * theta.cos()).hypot(dy - mid * theta.sin()) <= stroke / 2.0
        };
        if (in_band && angle <= value) || cap(0.0) || cap(value) {
            return Some(Part::Arc);
        }
    }
    ((radius - mid).abs() <= stroke * TRACK / 2.0).then_some(Part::Track)
}

pub struct Pixel {
    pub coverage: f64,
    pub alpha: u8,
}

pub fn raster(size: u32, glyph: Glyph) -> Vec<Pixel> {
    let extent = f64::from(size);
    let centre = extent / 2.0;
    let total = (SAMPLES * SAMPLES) as f64;
    let mut pixels = Vec::with_capacity((size * size) as usize);
    for y in 0..size {
        for x in 0..size {
            let (mut covered, mut alpha) = (0.0, 0.0);
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let dx = f64::from(x) + (sx as f64 + 0.5) / SAMPLES as f64 - centre;
                    let dy = f64::from(y) + (sy as f64 + 0.5) / SAMPLES as f64 - centre;
                    if let Some(part) = part_at(dx, dy, extent, glyph) {
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

    fn centre(size: u32, glyph: Glyph) -> u8 {
        raster(size, glyph)[(size / 2 * size + size / 2) as usize].alpha
    }

    fn top(size: u32, glyph: Glyph) -> u8 {
        let row = (f64::from(size) * (0.5 - OUTER + STROKE / 2.0)) as u32;
        raster(size, glyph)[(row * size + size / 2) as usize].alpha
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
    fn glyphs_are_distinct() {
        for size in SIZES {
            assert_eq!(centre(size, Glyph::Full), 0);
            assert_eq!(centre(size, Glyph::Held), 255);
            assert_eq!(top(size, Glyph::Full), 255);
            assert!(top(size, Glyph::Off) < 255);
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
    fn remaining_clamps_and_ends_match_off_and_full() {
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

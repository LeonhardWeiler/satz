use moxcms::{ColorProfile, Layout, RenderingIntent, TransformF32Executor, TransformOptions};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};

/// The print condition of CMYK documents; see `icc/build`.
const FOGRA51: &[u8] = include_bytes!("../icc/FOGRA51.icc");

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ColorMode {
    #[default]
    Rgb,
    Cmyk,
}

/// A colour keeps its own space whatever the document's mode.
/// RGB is 0xRRGGBBAA; CMYK components, tint and alpha are 0..=1.
/// The tint of a swatch only applies to spot colours.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Color {
    Rgb(u32),
    Cmyk {
        cmyk: [f32; 4],
        alpha: f32,
    },
    Swatch {
        swatch: String,
        tint: f32,
        alpha: f32,
    },
}

/// How a colour prints: as RGB, as CMYK, or as a tint of a spot colour.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Ink {
    #[default]
    Rgb,
    Cmyk([f32; 4]),
    Spot {
        name: String,
        cmyk: [f32; 4],
        tint: f32,
    },
}

/// A spot colour's `color` is its CMYK alternate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Swatch {
    pub id: String,
    pub name: String,
    pub color: Color,
    pub spot: bool,
}

impl Swatch {
    pub fn check(&self) -> Result<(), String> {
        self.color.check()?;
        match (&self.color, self.spot) {
            (Color::Swatch { .. }, _) => Err("a swatch cannot use a swatch".into()),
            (Color::Rgb(_), true) => Err("a spot colour needs a CMYK alternate".into()),
            _ => Ok(()),
        }
    }
}

impl From<u32> for Color {
    fn from(c: u32) -> Color {
        Color::Rgb(c)
    }
}

impl Color {
    pub fn check(&self) -> Result<(), String> {
        let unit = |v: &f32| (0.0..=1.0).contains(v);
        let ok = match self {
            Color::Rgb(_) => true,
            Color::Cmyk { cmyk, alpha } => cmyk.iter().chain([alpha]).all(unit),
            Color::Swatch { tint, alpha, .. } => unit(tint) && unit(alpha),
        };
        ok.then_some(())
            .ok_or_else(|| "colour values must be in 0..=1".into())
    }

    /// Black, white and light gray in the document's mode.
    pub fn black(mode: ColorMode) -> Color {
        Color::of(mode, 0x000000ff, 1.0)
    }

    pub fn white(mode: ColorMode) -> Color {
        Color::of(mode, 0xffffffff, 0.0)
    }

    pub fn gray(mode: ColorMode) -> Color {
        Color::of(mode, 0xd9d9d9ff, 0.15)
    }

    fn of(mode: ColorMode, rgb: u32, k: f32) -> Color {
        match mode {
            ColorMode::Rgb => Color::Rgb(rgb),
            ColorMode::Cmyk => Color::Cmyk {
                cmyk: [0.0, 0.0, 0.0, k],
                alpha: 1.0,
            },
        }
    }

    /// The process colour a swatch stands for; missing swatches are transparent.
    pub fn resolve(&self, swatches: &[Swatch]) -> Color {
        let Color::Swatch {
            swatch,
            tint,
            alpha,
        } = self
        else {
            return self.clone();
        };
        match swatches.iter().find(|s| s.id == *swatch) {
            Some(Swatch {
                color: Color::Cmyk { cmyk, alpha: a },
                spot: true,
                ..
            }) => Color::Cmyk {
                cmyk: cmyk.map(|v| v * tint),
                alpha: a * alpha,
            },
            Some(s) => match s.color {
                Color::Rgb(c) => {
                    let a = ((c & 0xff) as f32 * alpha).round() as u32;
                    Color::Rgb(c & !0xff | a)
                }
                Color::Cmyk { cmyk, alpha: a } => Color::Cmyk {
                    cmyk,
                    alpha: a * alpha,
                },
                Color::Swatch { .. } => Color::Rgb(0),
            },
            None => Color::Rgb(0),
        }
    }

    pub fn ink(&self, swatches: &[Swatch]) -> Ink {
        if let Color::Swatch { swatch, tint, .. } = self
            && let Some(Swatch {
                name,
                color: Color::Cmyk { cmyk, .. },
                spot: true,
                ..
            }) = swatches.iter().find(|s| s.id == *swatch)
        {
            return Ink::Spot {
                name: name.clone(),
                cmyk: *cmyk,
                tint: *tint,
            };
        }
        match self.resolve(swatches) {
            Color::Cmyk { cmyk, .. } => Ink::Cmyk(cmyk),
            _ => Ink::Rgb,
        }
    }

    /// Screen colour; CMYK is previewed through FOGRA51.
    pub fn rgba(&self, swatches: &[Swatch]) -> [f32; 4] {
        match self.resolve(swatches) {
            Color::Rgb(c) => c.to_be_bytes().map(|v| v as f32 / 255.0),
            Color::Cmyk { cmyk, alpha } => {
                let [r, g, b] = to_rgb(cmyk);
                [r, g, b, alpha]
            }
            Color::Swatch { .. } => [0.0; 4],
        }
    }
}

fn transform(
    from: &ColorProfile,
    from_layout: Layout,
    to: &ColorProfile,
    to_layout: Layout,
) -> Arc<TransformF32Executor> {
    let options = TransformOptions {
        rendering_intent: RenderingIntent::RelativeColorimetric,
        ..TransformOptions::default()
    };
    from.create_transform_f32(from_layout, to, to_layout, options)
        .unwrap()
}

fn fogra51() -> ColorProfile {
    ColorProfile::new_from_slice(FOGRA51).unwrap()
}

pub fn to_rgb(cmyk: [f32; 4]) -> [f32; 3] {
    static T: OnceLock<Arc<TransformF32Executor>> = OnceLock::new();
    let t = T.get_or_init(|| {
        transform(
            &fogra51(),
            Layout::Rgba,
            &ColorProfile::new_srgb(),
            Layout::Rgb,
        )
    });
    let mut rgb = [0.0; 3];
    t.transform(&cmyk, &mut rgb).unwrap();
    rgb.map(|v| v.clamp(0.0, 1.0))
}

pub fn to_cmyk(rgb: [f32; 3]) -> [f32; 4] {
    static T: OnceLock<Arc<TransformF32Executor>> = OnceLock::new();
    let t = T.get_or_init(|| {
        transform(
            &ColorProfile::new_srgb(),
            Layout::Rgb,
            &fogra51(),
            Layout::Rgba,
        )
    });
    let mut cmyk = [0.0; 4];
    t.transform(&rgb, &mut cmyk).unwrap();
    cmyk.map(|v| v.clamp(0.0, 1.0))
}

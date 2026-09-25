use crate::variable::{Scope, Value};
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
/// The tint of a swatch only applies to spot colours; alpha multiplies what a
/// swatch or variable holds.
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
    Variable {
        variable: String,
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

impl Ink {
    /// The process colour this prints as; `rgba` is its screen colour.
    pub fn cmyk(&self, rgba: &[f32; 4]) -> [f32; 4] {
        match self {
            Ink::Rgb => to_cmyk([rgba[0], rgba[1], rgba[2]]),
            Ink::Cmyk(c) => *c,
            Ink::Spot { cmyk, tint, .. } => cmyk.map(|v| v * tint),
        }
    }
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
            (Color::Swatch { .. } | Color::Variable { .. }, _) => {
                Err("a swatch cannot use a swatch or variable".into())
            }
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
            Color::Variable { alpha, .. } => unit(alpha),
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

    /// `self` with its alpha multiplied by `a`.
    fn fade(&self, a: f32) -> Color {
        match self.clone() {
            Color::Rgb(c) => Color::Rgb(c & !0xff | ((c & 0xff) as f32 * a).round() as u32),
            Color::Cmyk { cmyk, alpha } => Color::Cmyk {
                cmyk,
                alpha: alpha * a,
            },
            Color::Swatch {
                swatch,
                tint,
                alpha,
            } => Color::Swatch {
                swatch,
                tint,
                alpha: alpha * a,
            },
            Color::Variable { variable, alpha } => Color::Variable {
                variable,
                alpha: alpha * a,
            },
        }
    }

    /// The colour a variable holds in the scope's modes; missing variables are transparent.
    pub fn bind(&self, s: &Scope) -> Color {
        let Color::Variable { variable, alpha } = self else {
            return self.clone();
        };
        match s.value(variable) {
            Some(Value::Color(c)) if !matches!(c, Color::Variable { .. }) => c.fade(*alpha),
            _ => Color::Rgb(0),
        }
    }

    /// The process colour a swatch or variable stands for; missing swatches are transparent.
    pub fn resolve(&self, s: &Scope) -> Color {
        match self.bind(s) {
            Color::Swatch {
                swatch,
                tint,
                alpha,
            } => match s.swatches().iter().find(|w| w.id == swatch) {
                Some(Swatch {
                    color: Color::Cmyk { cmyk, alpha: a },
                    spot: true,
                    ..
                }) => Color::Cmyk {
                    cmyk: cmyk.map(|v| v * tint),
                    alpha: a * alpha,
                },
                Some(w) if matches!(w.color, Color::Rgb(_) | Color::Cmyk { .. }) => {
                    w.color.fade(alpha)
                }
                _ => Color::Rgb(0),
            },
            c => c,
        }
    }

    pub fn ink(&self, s: &Scope) -> Ink {
        if let Color::Swatch { swatch, tint, .. } = self.bind(s)
            && let Some(Swatch {
                name,
                color: Color::Cmyk { cmyk, .. },
                spot: true,
                ..
            }) = s.swatches().iter().find(|w| w.id == swatch)
        {
            return Ink::Spot {
                name: name.clone(),
                cmyk: *cmyk,
                tint,
            };
        }
        match self.resolve(s) {
            Color::Cmyk { cmyk, .. } => Ink::Cmyk(cmyk),
            _ => Ink::Rgb,
        }
    }

    /// Screen colour; CMYK is previewed through FOGRA51.
    pub fn rgba(&self, s: &Scope) -> [f32; 4] {
        match self.resolve(s) {
            Color::Rgb(c) => c.to_be_bytes().map(|v| v as f32 / 255.0),
            Color::Cmyk { cmyk, alpha } => {
                let [r, g, b] = to_rgb(cmyk);
                [r, g, b, alpha]
            }
            _ => [0.0; 4],
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

/// CMYK of the colour 0xRRGGBB; black, white and gray take their pure K.
pub fn separate(rgb: u32) -> [f32; 4] {
    let rgba = Color::Rgb(rgb << 8 | 0xff);
    for neutral in [Color::black, Color::white, Color::gray] {
        if let (true, Color::Cmyk { cmyk, .. }) =
            (neutral(ColorMode::Rgb) == rgba, neutral(ColorMode::Cmyk))
        {
            return cmyk;
        }
    }
    to_cmyk([16, 8, 0].map(|s| (rgb >> s & 0xff) as f32 / 255.0))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_white_and_gray_separate_to_pure_k() {
        assert_eq!(separate(0x000000), [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(separate(0xffffff), [0.0; 4]);
        assert_eq!(separate(0xd9d9d9), [0.0, 0.0, 0.0, 0.15]);
        let [c, m, y, _] = separate(0xff0000);
        assert!(c < 0.05 && m > 0.9 && y > 0.9);
    }
}

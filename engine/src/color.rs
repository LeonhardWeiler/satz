use crate::variable::{Scope, Value};
use moxcms::{
    ColorProfile, DataColorSpace, Layout, ProfileText, RenderingIntent, Transform8BitExecutor,
    TransformF32Executor, TransformOptions,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/// The print condition of CMYK documents; see `icc/build`.
const FOGRA51: &[u8] = include_bytes!("../icc/FOGRA51.icc");

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ColorMode {
    #[default]
    Rgb,
    Cmyk,
}

/// Switching the document's mode converts process colours into it.
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

    /// `self` as a process colour of `mode`, or `None` if it is one or names a
    /// swatch or variable.
    pub fn convert(&self, mode: ColorMode) -> Option<Color> {
        match (self, mode) {
            (Color::Rgb(c), ColorMode::Cmyk) => Some(Color::Cmyk {
                cmyk: separate(c >> 8),
                alpha: (c & 0xff) as f32 / 255.0,
            }),
            (Color::Cmyk { cmyk, alpha }, ColorMode::Rgb) => {
                let [r, g, b] = to_rgb(*cmyk);
                Some(Color::Rgb(u32::from_be_bytes(
                    [r, g, b, *alpha].map(|v| (v * 255.0).round() as u8),
                )))
            }
            _ => None,
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

fn options() -> TransformOptions {
    TransformOptions {
        rendering_intent: RenderingIntent::RelativeColorimetric,
        ..TransformOptions::default()
    }
}

/// The CMYK profile of the document and its transforms to and from sRGB.
struct Cms {
    icc: Vec<u8>,
    name: String,
    to_srgb: Arc<TransformF32Executor>,
    from_srgb: Arc<TransformF32Executor>,
    separate: Arc<Transform8BitExecutor>,
    preview: Arc<Transform8BitExecutor>,
    /// Black point compensation, as PDF viewers apply it: the darkest colour the
    /// profile prints shows as screen black.
    lut: [[u8; 256]; 3],
    black: [f32; 3],
}

impl Cms {
    fn new(icc: &[u8]) -> Result<Cms, String> {
        let p = ColorProfile::new_from_slice(icc).map_err(|_| "not an ICC profile")?;
        if p.color_space != DataColorSpace::Cmyk {
            return Err("not a CMYK profile".into());
        }
        let srgb = ColorProfile::new_srgb();
        let fail = |_| "the profile does not convert colours".to_string();
        let name = match &p.description {
            Some(ProfileText::PlainString(s)) => s.clone(),
            Some(ProfileText::Localizable(l)) => {
                l.first().map(|l| l.value.clone()).unwrap_or_default()
            }
            Some(ProfileText::Description(d)) => d.ascii_string.clone(),
            None => String::new(),
        };
        let mut c = Cms {
            icc: icc.to_vec(),
            name: name.trim_end_matches('\0').trim().to_string(),
            to_srgb: p
                .create_transform_f32(Layout::Rgba, &srgb, Layout::Rgb, options())
                .map_err(fail)?,
            from_srgb: srgb
                .create_transform_f32(Layout::Rgb, &p, Layout::Rgba, options())
                .map_err(fail)?,
            separate: srgb
                .create_transform_8bit(Layout::Rgb, &p, Layout::Rgba, options())
                .map_err(fail)?,
            preview: p
                .create_transform_8bit(Layout::Rgba, &srgb, Layout::Rgb, options())
                .map_err(fail)?,
            lut: [[0; 256]; 3],
            black: [0.0; 3],
        };
        c.black = c.srgb(c.cmyk([0.0; 3])).map(linear);
        c.lut = std::array::from_fn(|i| {
            std::array::from_fn(|v| {
                let mut rgb = [0.0; 3];
                rgb[i] = v as f32 / 255.0;
                (c.compensate(rgb)[i] * 255.0).round() as u8
            })
        });
        Ok(c)
    }

    fn srgb(&self, cmyk: [f32; 4]) -> [f32; 3] {
        let mut rgb = [0.0; 3];
        self.to_srgb.transform(&cmyk, &mut rgb).unwrap();
        rgb.map(|v| v.clamp(0.0, 1.0))
    }

    fn cmyk(&self, rgb: [f32; 3]) -> [f32; 4] {
        let mut cmyk = [0.0; 4];
        self.from_srgb.transform(&rgb, &mut cmyk).unwrap();
        cmyk.map(|v| v.clamp(0.0, 1.0))
    }

    fn compensate(&self, rgb: [f32; 3]) -> [f32; 3] {
        let b = self.black;
        std::array::from_fn(|i| gamma(((linear(rgb[i]) - b[i]) / (1.0 - b[i])).clamp(0.0, 1.0)))
    }
}

thread_local! {
    static CMS: RefCell<Rc<Cms>> = RefCell::new(Rc::new(Cms::new(FOGRA51).unwrap()));
}

fn cms() -> Rc<Cms> {
    CMS.with(|c| c.borrow().clone())
}

/// Checks that `icc` is a CMYK profile that converts colours.
pub fn check_profile(icc: &[u8]) -> Result<(), String> {
    Cms::new(icc).map(|_| ())
}

/// The name of the CMYK profile in use, `None` for FOGRA51.
pub fn profile_name() -> Option<String> {
    let c = cms();
    (c.icc != FOGRA51).then(|| c.name.clone())
}

/// Makes `icc`, or FOGRA51 for `None`, the CMYK profile of the colours that follow;
/// true when it changed.
pub fn use_profile(icc: Option<&[u8]>) -> bool {
    let icc = icc.unwrap_or(FOGRA51);
    let Some(c) = (cms().icc != icc).then(|| Cms::new(icc).ok()).flatten() else {
        return false;
    };
    CMS.with(|cell| *cell.borrow_mut() = Rc::new(c));
    true
}

/// The CMYK profile in use: its bytes, its name, and whether it is the built-in FOGRA51.
pub fn profile() -> (Vec<u8>, String, bool) {
    let c = cms();
    (c.icc.clone(), c.name.clone(), c.icc == FOGRA51)
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

/// Screen colour of `cmyk`, black point compensated.
pub fn to_rgb(cmyk: [f32; 4]) -> [f32; 3] {
    let c = cms();
    c.compensate(c.srgb(cmyk))
}

pub fn to_cmyk(rgb: [f32; 3]) -> [f32; 4] {
    cms().cmyk(rgb)
}

fn linear(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn gamma(v: f32) -> f32 {
    if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

/// The colour `rgb` prints visibly different in the CMYK profile.
pub fn out_of_gamut(rgb: [f32; 3]) -> bool {
    apart(rgb, to_rgb(to_cmyk(rgb)))
}

/// The sRGB colours `a` and `b` differ in chroma by more than 10 in CIELAB.
pub fn apart(a: [f32; 3], b: [f32; 3]) -> bool {
    let (a, b) = (chroma(a), chroma(b));
    (a[0] - b[0]).hypot(a[1] - b[1]) > 10.0
}

/// a* and b* of the sRGB colour `rgb` in CIELAB under D65.
fn chroma(rgb: [f32; 3]) -> [f32; 2] {
    let [r, g, b] = rgb.map(linear);
    let xyz = [
        (0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.9505,
        0.2126 * r + 0.7152 * g + 0.0722 * b,
        (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.089,
    ]
    .map(|t| {
        if t > 0.008856 {
            t.cbrt()
        } else {
            7.787 * t + 16.0 / 116.0
        }
    });
    [500.0 * (xyz[0] - xyz[1]), 200.0 * (xyz[1] - xyz[2])]
}

/// CMYK bytes of the RGBA bytes `rgba`, whose alpha is left out.
pub fn separate_pixels(rgba: &[u8]) -> Vec<u8> {
    let rgb: Vec<u8> = rgba.chunks(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
    let mut cmyk = vec![0; rgb.len() / 3 * 4];
    cms().separate.transform(&rgb, &mut cmyk).unwrap();
    cmyk
}

/// Screen RGB bytes of the CMYK bytes `cmyk`, compensated as `to_rgb` is.
pub fn preview_pixels(cmyk: &[u8]) -> Vec<u8> {
    let c = cms();
    let mut rgb = vec![0; cmyk.len() / 4 * 3];
    c.preview.transform(cmyk, &mut rgb).unwrap();
    for (i, v) in rgb.iter_mut().enumerate() {
        *v = c.lut[i % 3][*v as usize];
    }
    rgb
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

    #[test]
    fn the_darkest_print_colour_previews_as_black_and_paper_as_white() {
        let bytes = |cmyk| to_rgb(cmyk).map(|v| (v * 255.0).round());
        assert_eq!(bytes(to_cmyk([0.0; 3])), [0.0; 3]);
        assert_eq!(bytes([0.0; 4]), [255.0; 3]);
        assert!(
            to_rgb([0.0, 0.0, 0.0, 1.0])
                .iter()
                .all(|&v| v > 0.0 && v < 0.1)
        );
    }

    #[test]
    fn pixels_preview_as_single_colours_do() {
        let cmyk: [f32; 4] = [0.1, 0.6, 0.9, 0.05];
        let rgb = preview_pixels(&cmyk.map(|v| (v * 255.0).round() as u8));
        let one = to_rgb(cmyk).map(|v| (v * 255.0).round() as i32);
        assert!(
            rgb.iter().zip(one).all(|(&a, b)| (a as i32 - b).abs() <= 2),
            "{rgb:?} vs {one:?}"
        );
    }

    #[test]
    fn pixels_separate_as_single_colours_do() {
        let cmyk = separate_pixels(&[255, 0, 0, 255, 0, 0, 0, 0]);
        let red = separate(0xff0000).map(|v| (v * 255.0).round() as i32);
        assert!(
            cmyk[..4]
                .iter()
                .zip(red)
                .all(|(&a, b)| (a as i32 - b).abs() <= 2),
            "{cmyk:?} vs {red:?}"
        );
        assert!(cmyk[7] > 200);
    }
}

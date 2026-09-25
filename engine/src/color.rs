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
/// RGB is 0xRRGGBBAA; CMYK components and alpha are 0..=1.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Color {
    Rgb(u32),
    Cmyk { cmyk: [f32; 4], alpha: f32 },
}

impl From<u32> for Color {
    fn from(c: u32) -> Color {
        Color::Rgb(c)
    }
}

impl Color {
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

    /// Screen colour; CMYK is previewed through FOGRA51.
    pub fn rgba(&self) -> [f32; 4] {
        match *self {
            Color::Rgb(c) => c.to_be_bytes().map(|v| v as f32 / 255.0),
            Color::Cmyk { cmyk, alpha } => {
                let [r, g, b] = to_rgb(cmyk);
                [r, g, b, alpha]
            }
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

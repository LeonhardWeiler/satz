use crate::color::{preview_pixels, separate_pixels};
use crate::content_hash;
use krilla::Data;
use krilla::image::{BitsPerComponent, CustomImage, ImageColorspace};
use loro::LoroBinaryValue;
use serde::Serialize;
use std::cell::RefCell;
use std::io::Cursor;
use std::rc::Rc;
use std::sync::Arc;
use tiny_skia::{IntSize, Pixmap};
use zune_core::colorspace::ColorSpace;
use zune_core::options::DecoderOptions;
use zune_jpeg::JpegDecoder;

/// A PNG or JPEG file by the hash of its bytes, its size in pixels and its colours.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImageInfo {
    pub hash: String,
    pub width: u32,
    pub height: u32,
    pub space: Space,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum Space {
    Gray,
    #[serde(rename = "RGB")]
    Rgb,
    #[serde(rename = "CMYK")]
    Cmyk,
}

#[derive(Clone, Copy, PartialEq)]
enum Format {
    Png,
    Jpeg,
}

struct Entry {
    info: ImageInfo,
    format: Format,
    bytes: LoroBinaryValue,
    /// The image and adjustment an adjusted image is made of; its `bytes` are empty.
    source: Option<(Rc<Entry>, [f32; 3])>,
    /// Decoded on first use by `pixmap` and `cmyk`; `None` when the file does not decode.
    pixels: RefCell<Option<Option<Rc<Pixmap>>>>,
    cmyk: RefCell<Option<Option<Cmyk>>>,
    /// `cmyk` shrunk by the factor `plate` last needed.
    small: RefCell<Option<(u32, Cmyk)>>,
}

/// CMYK pixels and their alpha, 8 bits each, as the PDF of a CMYK document takes them.
#[derive(Hash, Clone)]
pub struct Cmyk {
    pub color: Arc<[u8]>,
    pub alpha: Arc<[u8]>,
    pub size: (u32, u32),
}

impl CustomImage for Cmyk {
    fn color_channel(&self) -> &[u8] {
        &self.color
    }

    fn alpha_channel(&self) -> Option<&[u8]> {
        self.alpha.iter().any(|&a| a < 255).then_some(&self.alpha)
    }

    fn bits_per_component(&self) -> BitsPerComponent {
        BitsPerComponent::Eight
    }

    fn size(&self) -> (u32, u32) {
        self.size
    }

    fn icc_profile(&self) -> Option<&[u8]> {
        None
    }

    fn color_space(&self) -> ImageColorspace {
        ImageColorspace::Cmyk
    }
}

/// Mark the id of an image in a display list that shows it as it prints, or as
/// the inverted C, M and Y or K plate that `Doc::plate` draws.
pub const PROOF: u32 = 1 << 31;
pub const CMY: u32 = 1 << 30;
pub const K: u32 = 1 << 29;

thread_local! {
    /// The images of every document loaded so far by id, so that ids stay valid
    /// across documents for the renderer's cache.
    static IMAGES: RefCell<Vec<Rc<Entry>>> = const { RefCell::new(Vec::new()) };
}

fn format(bytes: &[u8]) -> Result<Format, String> {
    match bytes {
        [0x89, b'P', b'N', b'G', ..] => Ok(Format::Png),
        [0xff, 0xd8, 0xff, ..] => Ok(Format::Jpeg),
        _ => Err("not a PNG or JPEG image".into()),
    }
}

fn pdf_image(format: Format, bytes: &LoroBinaryValue) -> Result<krilla::image::Image, String> {
    let data =
        Data::from(std::sync::Arc::new(Shared(bytes.clone()))
            as std::sync::Arc<dyn AsRef<[u8]> + Send + Sync>);
    match format {
        Format::Png => krilla::image::Image::from_png(data, true),
        Format::Jpeg => krilla::image::Image::from_jpeg(data, true),
    }
}

/// Lends the document's image bytes to the PDF without copying them.
struct Shared(LoroBinaryValue);

impl AsRef<[u8]> for Shared {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Registers the image `bytes` unless it is there already; fails when they are not
/// a PNG or JPEG file.
pub fn register(bytes: LoroBinaryValue) -> Result<ImageInfo, String> {
    let hash = content_hash(&bytes);
    if let Some(e) = entry_by_hash(&hash) {
        return Ok(e.info.clone());
    }
    let format = format(&bytes)?;
    let (width, height) = pdf_image(format, &bytes)
        .map_err(|_| "not a PNG or JPEG image")?
        .size();
    let info = ImageInfo {
        hash,
        width,
        height,
        space: space(format, &bytes),
    };
    IMAGES.with_borrow_mut(|images| {
        images.push(Rc::new(Entry {
            info: info.clone(),
            format,
            bytes,
            source: None,
            pixels: RefCell::new(None),
            cmyk: RefCell::new(None),
            small: RefCell::new(None),
        }))
    });
    Ok(info)
}

/// The width and height in the header of the PNG or JPEG file `bytes`.
pub fn size(bytes: &[u8]) -> Result<(u32, u32), String> {
    let bad = || "not a PNG or JPEG image".to_string();
    match format(bytes)? {
        Format::Png => {
            let r = png::Decoder::new(Cursor::new(bytes))
                .read_info()
                .map_err(|_| bad())?;
            Ok((r.info().width, r.info().height))
        }
        Format::Jpeg => {
            let mut d = JpegDecoder::new(Cursor::new(bytes));
            d.decode_headers().map_err(|_| bad())?;
            let (w, h) = d.dimensions().ok_or_else(bad)?;
            Ok((w as u32, h as u32))
        }
    }
}

/// The id of the registered image `hash` with its brightness, contrast and
/// saturation changed by `adjust`, each in -1..=1. Making one drops the decoded
/// pixels of the other adjustments of `hash`.
pub fn adjusted(hash: &str, adjust: [f32; 3]) -> Option<u32> {
    if adjust == [0.0; 3] {
        return id(hash);
    }
    let key = format!("{hash}{adjust:?}");
    if let Some(i) = id(&key) {
        return Some(i);
    }
    let source = entry_by_hash(hash)?;
    IMAGES.with_borrow_mut(|images| {
        for e in images.iter().filter(|e| {
            e.source
                .as_ref()
                .is_some_and(|(s, _)| Rc::ptr_eq(s, &source))
        }) {
            e.pixels.take();
            e.cmyk.take();
            e.small.take();
        }
        images.push(Rc::new(Entry {
            info: ImageInfo {
                hash: key,
                space: Space::Rgb,
                ..source.info.clone()
            },
            format: Format::Png,
            bytes: Vec::new().into(),
            source: Some((source, adjust)),
            pixels: RefCell::new(None),
            cmyk: RefCell::new(None),
            small: RefCell::new(None),
        }));
        Some(images.len() as u32 - 1)
    })
}

/// Drops the decoded pixels of every image but `keep`.
pub fn forget(keep: &[String]) {
    IMAGES.with_borrow(|images| {
        for e in images.iter().filter(|e| !keep.contains(&e.info.hash)) {
            e.pixels.take();
            e.cmyk.take();
        }
    })
}

fn space(format: Format, bytes: &[u8]) -> Space {
    match format {
        Format::Png => match png::Decoder::new(Cursor::new(bytes)).read_info() {
            Ok(r)
                if matches!(
                    r.info().color_type,
                    png::ColorType::Grayscale | png::ColorType::GrayscaleAlpha
                ) =>
            {
                Space::Gray
            }
            _ => Space::Rgb,
        },
        Format::Jpeg => {
            let mut decoder = JpegDecoder::new(Cursor::new(bytes));
            match decoder
                .decode_headers()
                .ok()
                .and(decoder.input_colorspace())
            {
                Some(ColorSpace::CMYK | ColorSpace::YCCK) => Space::Cmyk,
                Some(ColorSpace::Luma | ColorSpace::LumaA) => Space::Gray,
                _ => Space::Rgb,
            }
        }
    }
}

fn entry_by_hash(hash: &str) -> Option<Rc<Entry>> {
    IMAGES.with_borrow(|images| images.iter().find(|e| e.info.hash == hash).cloned())
}

fn entry(id: u32) -> Option<Rc<Entry>> {
    IMAGES.with_borrow(|images| images.get((id & !(PROOF | CMY | K)) as usize).cloned())
}

/// The id of the registered image `hash` in display lists.
pub fn id(hash: &str) -> Option<u32> {
    IMAGES.with_borrow(|images| {
        images
            .iter()
            .position(|e| e.info.hash == hash)
            .map(|i| i as u32)
    })
}

/// The size of the registered image `hash` in pixels.
pub fn info(hash: &str) -> Option<ImageInfo> {
    entry_by_hash(hash).map(|e| e.info.clone())
}

/// The file of the image `id`, or a PNG of it for a marked id; empty for an
/// unknown id.
pub fn bytes(id: u32) -> Vec<u8> {
    if id & (PROOF | CMY | K) == 0 {
        return entry(id).and_then(|e| file(&e)).unwrap_or_default();
    }
    let proof = || {
        let c = cmyk(id)?;
        let rgb: Vec<u8> = match id & K != 0 {
            true => c.color.chunks(4).flat_map(|p| [255 - p[3]; 3]).collect(),
            false if id & CMY != 0 => c
                .color
                .chunks(4)
                .flat_map(|p| [255 - p[0], 255 - p[1], 255 - p[2]])
                .collect(),
            false => preview_pixels(&c.color),
        };
        let rgba: Vec<u8> = rgb
            .chunks(3)
            .zip(c.alpha.iter())
            .flat_map(|(p, &a)| [p[0], p[1], p[2], a])
            .collect();
        encode(&rgba, c.size)
    };
    proof().unwrap_or_default()
}

/// The PNG or JPEG file of `e`, encoded anew for an adjusted image.
fn file(e: &Entry) -> Option<Vec<u8>> {
    match e.source {
        Some(_) => {
            let (rgba, w, h) = rgba(e)?;
            encode(&rgba, (w, h))
        }
        None => Some(e.bytes.to_vec()),
    }
}

pub fn encode(rgba: &[u8], (w, h): (u32, u32)) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_compression(png::Compression::Fastest);
    let mut writer = encoder.write_header().ok()?;
    writer.write_image_data(rgba).ok()?;
    writer.finish().ok()?;
    Some(out)
}

/// The image `id` for the PDF, which embeds a JPEG as it is, separates an RGB
/// image through FOGRA51 for `cmyk`, or shows it as it prints for a `PROOF` id.
pub fn pdf(id: u32, cmyk: bool) -> Option<krilla::image::Image> {
    let e = entry(id)?;
    if e.source.is_none() && id & PROOF == 0 && (!cmyk || e.info.space == Space::Cmyk) {
        return pdf_image(e.format, &e.bytes).ok();
    }
    if cmyk {
        return krilla::image::Image::from_custom(self::cmyk(id)?, true).ok();
    }
    krilla::image::Image::from_png(bytes(id).into(), true).ok()
}

/// The pixels of the image `id`, premultiplied, for rasterized effects.
pub fn pixmap(id: u32) -> Option<Rc<Pixmap>> {
    let e = entry(id)?;
    let mut pixels = e.pixels.borrow_mut();
    pixels
        .get_or_insert_with(|| {
            let (rgba, w, h) = rgba(&e)?;
            let premultiplied = rgba
                .chunks(4)
                .flat_map(|p| {
                    let a = p[3] as u32;
                    let m = |c: u8| ((c as u32 * a + 127) / 255) as u8;
                    [m(p[0]), m(p[1]), m(p[2]), p[3]]
                })
                .collect();
            Pixmap::from_vec(premultiplied, IntSize::from_wh(w, h)?).map(Rc::new)
        })
        .clone()
}

/// The pixels of the image `id` in CMYK: a CMYK JPEG's own, others separated
/// through FOGRA51.
pub fn cmyk(id: u32) -> Option<Cmyk> {
    let e = entry(id)?;
    let mut cmyk = e.cmyk.borrow_mut();
    cmyk.get_or_insert_with(|| {
        if e.info.space == Space::Cmyk {
            let (color, w, h) = decode_cmyk(&e.bytes)?;
            return Some(Cmyk {
                color: color.into(),
                alpha: vec![255; (w * h) as usize].into(),
                size: (w, h),
            });
        }
        let (rgba, w, h) = rgba(&e)?;
        Some(Cmyk {
            color: separate_pixels(&rgba).into(),
            alpha: rgba.chunks(4).map(|p| p[3]).collect(),
            size: (w, h),
        })
    })
    .clone()
}

/// The pixels of `e` as RGBA, not premultiplied, and its size.
fn rgba(e: &Entry) -> Option<(Vec<u8>, u32, u32)> {
    let Some((source, [b, c, s])) = &e.source else {
        return decode(e.format, &e.bytes);
    };
    let (mut rgba, w, h) = rgba(source)?;
    for p in rgba.chunks_mut(4) {
        let [r, g, bl] = [p[0], p[1], p[2]].map(|v| v as f32 / 255.0);
        let l = 0.2126 * r + 0.7152 * g + 0.0722 * bl;
        for v in &mut p[..3] {
            let x = l + (*v as f32 / 255.0 - l) * (1.0 + s);
            let x = (x - 0.5) * (1.0 + c) + 0.5 + b;
            *v = (x.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
    Some((rgba, w, h))
}

/// The pixels of a PNG or JPEG file as RGBA, not premultiplied, and its size.
fn decode(format: Format, bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    Some(match format {
        Format::Png => {
            let mut decoder = png::Decoder::new(Cursor::new(bytes));
            decoder.set_transformations(png::Transformations::normalize_to_color8());
            let mut reader = decoder.read_info().ok()?;
            let mut buf = vec![0; reader.output_buffer_size()?];
            let frame = reader.next_frame(&mut buf).ok()?;
            let rgba: Vec<u8> = match frame.color_type {
                png::ColorType::Rgba => buf,
                png::ColorType::Rgb => buf
                    .chunks(3)
                    .flat_map(|p| [p[0], p[1], p[2], 255])
                    .collect(),
                png::ColorType::GrayscaleAlpha => buf
                    .chunks(2)
                    .flat_map(|p| [p[0], p[0], p[0], p[1]])
                    .collect(),
                png::ColorType::Grayscale => buf.iter().flat_map(|&g| [g, g, g, 255]).collect(),
                png::ColorType::Indexed => return None,
            };
            (rgba, frame.width, frame.height)
        }
        Format::Jpeg => {
            let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGBA);
            let mut decoder = JpegDecoder::new_with_options(Cursor::new(bytes), options);
            let rgba = decoder.decode().ok()?;
            let (w, h) = decoder.dimensions()?;
            (rgba, w as u32, h as u32)
        }
    })
}

/// The inks of a CMYK or YCCK JPEG, which store them inverted as Adobe writes them,
/// and its size.
fn decode_cmyk(bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let mut decoder = JpegDecoder::new(Cursor::new(bytes));
    decoder.decode_headers().ok()?;
    let space = decoder.input_colorspace()?;
    let options = DecoderOptions::default().jpeg_set_out_colorspace(space);
    let mut decoder = JpegDecoder::new_with_options(Cursor::new(bytes), options);
    let raw = decoder.decode().ok()?;
    let (w, h) = decoder.dimensions()?;
    let ink = |v: f32| v.round().clamp(0.0, 255.0) as u8;
    let color = match space {
        ColorSpace::CMYK => raw.iter().map(|v| 255 - v).collect(),
        ColorSpace::YCCK => raw
            .chunks(4)
            .flat_map(|p| {
                let [y, cb, cr] = [p[0], p[1], p[2]].map(f32::from);
                let (cb, cr) = (cb - 128.0, cr - 128.0);
                [
                    ink(y + 1.402 * cr),
                    ink(y - 0.344136 * cb - 0.714136 * cr),
                    ink(y + 1.772 * cb),
                    255 - p[3],
                ]
            })
            .collect(),
        _ => return None,
    };
    Some((color, w as u32, h as u32))
}

/// The plate of the image `id` with the CMYK channels `pick` takes as RGB,
/// premultiplied, shrunk by a whole factor to no less than `size` pixels.
pub fn plate(id: u32, [w, h]: [f32; 2], pick: fn([f32; 4]) -> [f32; 3]) -> Option<Rc<Pixmap>> {
    let c = cmyk(id)?;
    let (cw, ch) = c.size;
    let f = (cw as f32 / w)
        .min(ch as f32 / h)
        .clamp(1.0, cw.min(ch) as f32) as u32;
    let c = if f == 1 {
        c
    } else {
        let e = entry(id)?;
        let mut small = e.small.borrow_mut();
        match &*small {
            Some((g, s)) if *g == f => s.clone(),
            _ => small.insert((f, shrink(&c, f))).1.clone(),
        }
    };
    let data = c
        .color
        .chunks(4)
        .zip(c.alpha.iter())
        .flat_map(|(p, &a)| {
            let [x, y, z] = pick([p[0], p[1], p[2], p[3]].map(|v| v as f32 / 255.0));
            let a = a as f32 / 255.0;
            [x * a, y * a, z * a, a].map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
        })
        .collect();
    Pixmap::from_vec(data, IntSize::from_wh(c.size.0, c.size.1)?).map(Rc::new)
}

/// `c` with each square of `f` by `f` pixels averaged into one.
fn shrink(c: &Cmyk, f: u32) -> Cmyk {
    let (w, h) = (c.size.0 / f, c.size.1 / f);
    let n = f * f;
    let (mut color, mut alpha) = (Vec::new(), Vec::new());
    for y in 0..h {
        for x in 0..w {
            let mut sum = [0u32; 5];
            for i in (y * f..(y + 1) * f)
                .flat_map(|v| (x * f..(x + 1) * f).map(move |u| (v * c.size.0 + u) as usize))
            {
                let px = c.color[4 * i..4 * i + 4].iter().chain([&c.alpha[i]]);
                for (s, &v) in sum.iter_mut().zip(px) {
                    *s += v as u32;
                }
            }
            let [a, b, d, e, g] = sum.map(|s| ((s + n / 2) / n) as u8);
            color.extend([a, b, d, e]);
            alpha.push(g);
        }
    }
    Cmyk {
        color: color.into(),
        alpha: alpha.into(),
        size: (w, h),
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// A PNG of `w` × `h` opaque red pixels.
    pub fn png(w: u32, h: u32) -> Vec<u8> {
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, w, h);
        encoder.set_color(png::ColorType::Rgb);
        let mut writer = encoder.write_header().unwrap();
        let data: Vec<u8> = (0..w * h).flat_map(|_| [255, 0, 0]).collect();
        writer.write_image_data(&data).unwrap();
        writer.finish().unwrap();
        out
    }

    #[test]
    fn an_image_is_registered_once_by_its_hash_and_decodes() {
        let info = register(png(3, 2).into()).unwrap();
        assert_eq!((info.width, info.height), (3, 2));
        assert_eq!(register(png(3, 2).into()).unwrap(), info);
        let id = id(&info.hash).unwrap();
        assert_eq!(bytes(id), png(3, 2));
        let proof = pixmap_of(&bytes(id | PROOF));
        assert!(proof.red() > 200 && proof.green() < 100, "{proof:?}");
        let [c, m, y, k] = [0, 1, 2, 3].map(|i| cmyk(id).unwrap().color[i]);
        let cmy = pixmap_of(&bytes(id | CMY));
        assert_eq!(
            [cmy.red(), cmy.green(), cmy.blue()],
            [255 - c, 255 - m, 255 - y]
        );
        let black = pixmap_of(&bytes(id | K));
        assert_eq!([black.red(), black.green(), black.blue()], [255 - k; 3]);
        let px = pixmap(id).unwrap();
        assert_eq!((px.width(), px.height()), (3, 2));
        assert_eq!(px.pixel(0, 0).unwrap().red(), 255);
    }

    fn pixmap_of(png: &[u8]) -> tiny_skia::ColorU8 {
        let (rgba, ..) = decode(Format::Png, png).unwrap();
        tiny_skia::ColorU8::from_rgba(rgba[0], rgba[1], rgba[2], rgba[3])
    }

    #[test]
    fn a_cmyk_jpeg_keeps_its_own_inks() {
        for (file, want) in [
            (
                &include_bytes!("../testdata/cmyk-black.jpg")[..],
                [0, 0, 0, 255],
            ),
            (include_bytes!("../testdata/ycck-cyan.jpg"), [255, 0, 0, 0]),
        ] {
            let info = register(file.to_vec().into()).unwrap();
            assert_eq!(info.space, Space::Cmyk);
            let c = cmyk(id(&info.hash).unwrap()).unwrap();
            assert!(
                c.color[..4]
                    .iter()
                    .zip(want)
                    .all(|(&a, b)| a.abs_diff(b) <= 2),
                "{:?}",
                &c.color[..4]
            );
        }
    }

    #[test]
    fn forgotten_images_drop_their_pixels() {
        let info = register(png(3, 1).into()).unwrap();
        let e = entry(id(&info.hash).unwrap()).unwrap();
        pixmap(id(&info.hash).unwrap()).unwrap();
        forget(std::slice::from_ref(&info.hash));
        assert!(e.pixels.borrow().is_some());
        forget(&[]);
        assert!(e.pixels.borrow().is_none());
    }

    #[test]
    fn adjusted_images_change_their_pixels_and_keep_the_pixels_of_one_adjustment() {
        let hash = register(png(4, 1).into()).unwrap().hash;
        assert_eq!(adjusted(&hash, [0.0; 3]), id(&hash));
        let dark = adjusted(&hash, [-0.5, 0.0, 0.0]).unwrap();
        assert_eq!(adjusted(&hash, [-0.5, 0.0, 0.0]), Some(dark));
        assert_eq!(pixmap_of(&bytes(dark)).red(), 128);
        assert_eq!(pixmap(dark).unwrap().pixel(0, 0).unwrap().red(), 128);
        let gray = adjusted(&hash, [0.0, 0.0, -1.0]).unwrap();
        let p = pixmap_of(&bytes(gray));
        assert_eq!([p.red(), p.green(), p.blue()], [54; 3]);
        assert!(entry(dark).unwrap().pixels.borrow().is_none());
        assert!(pdf(gray, false).is_some() && cmyk(gray).is_some());
    }

    #[test]
    fn other_files_are_not_images() {
        assert!(register(b"GIF89a".to_vec().into()).is_err());
        assert!(register(b"\x89PNG broken".to_vec().into()).is_err());
    }
}

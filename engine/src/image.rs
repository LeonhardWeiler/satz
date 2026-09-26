use crate::content_hash;
use krilla::Data;
use loro::LoroBinaryValue;
use serde::Serialize;
use std::cell::RefCell;
use std::io::Cursor;
use std::rc::Rc;
use tiny_skia::{IntSize, Pixmap};
use zune_core::colorspace::ColorSpace;
use zune_core::options::DecoderOptions;
use zune_jpeg::JpegDecoder;

/// A PNG or JPEG file by the hash of its bytes, and its size in pixels.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImageInfo {
    pub hash: String,
    pub width: u32,
    pub height: u32,
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
    /// Decoded on first use by `pixmap`; `None` when the file does not decode.
    pixels: RefCell<Option<Option<Rc<Pixmap>>>>,
}

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
    };
    IMAGES.with_borrow_mut(|images| {
        images.push(Rc::new(Entry {
            info: info.clone(),
            format,
            bytes,
            pixels: RefCell::new(None),
        }))
    });
    Ok(info)
}

fn entry_by_hash(hash: &str) -> Option<Rc<Entry>> {
    IMAGES.with_borrow(|images| images.iter().find(|e| e.info.hash == hash).cloned())
}

fn entry(id: u32) -> Option<Rc<Entry>> {
    IMAGES.with_borrow(|images| images.get(id as usize).cloned())
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

/// The file of the image `id`, empty for an unknown id.
pub fn bytes(id: u32) -> LoroBinaryValue {
    entry(id).map(|e| e.bytes.clone()).unwrap_or_default()
}

/// The image `id` for the PDF, which embeds a JPEG as it is.
pub fn pdf(id: u32) -> Option<krilla::image::Image> {
    let e = entry(id)?;
    pdf_image(e.format, &e.bytes).ok()
}

/// The pixels of the image `id`, premultiplied, for rasterized effects.
pub fn pixmap(id: u32) -> Option<Rc<Pixmap>> {
    let e = entry(id)?;
    let mut pixels = e.pixels.borrow_mut();
    pixels
        .get_or_insert_with(|| decode(e.format, &e.bytes).map(Rc::new))
        .clone()
}

fn decode(format: Format, bytes: &[u8]) -> Option<Pixmap> {
    let (rgba, w, h) = match format {
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
    };
    let premultiplied = rgba
        .chunks(4)
        .flat_map(|p| {
            let a = p[3] as u32;
            let m = |c: u8| ((c as u32 * a + 127) / 255) as u8;
            [m(p[0]), m(p[1]), m(p[2]), p[3]]
        })
        .collect();
    Pixmap::from_vec(premultiplied, IntSize::from_wh(w, h)?)
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
        assert_eq!(*bytes(id), png(3, 2));
        let px = pixmap(id).unwrap();
        assert_eq!((px.width(), px.height()), (3, 2));
        assert_eq!(px.pixel(0, 0).unwrap().red(), 255);
    }

    #[test]
    fn other_files_are_not_images() {
        assert!(register(b"GIF89a".to_vec().into()).is_err());
        assert!(register(b"\x89PNG broken".to_vec().into()).is_err());
    }
}

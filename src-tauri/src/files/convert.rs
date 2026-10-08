//! Shelf tools: convert or shrink images, merge PDFs. New files are written next to the
//! originals with a fresh name; originals are never changed.

use std::{
    io::BufWriter,
    path::{Path, PathBuf},
};

use image::{codecs::jpeg::JpegEncoder, DynamicImage, ImageFormat, ImageReader};
use lopdf::{Document, Object, ObjectId};

use super::unique_path;
use crate::error::{AppError, AppResult};

/// Formats images can be converted to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ImageTarget {
    Png,
    Jpg,
    Webp,
}

impl ImageTarget {
    fn ext(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpg => "jpg",
            Self::Webp => "webp",
        }
    }
}

const JPEG_QUALITY: u8 = 88;
const SHRINK_QUALITY: u8 = 78;
/// Shrunk images are at most this many pixels on the long side (and at most half size).
const SHRINK_MAX_SIDE: u32 = 2560;
/// Refuse absurd inputs (decompression bombs).
const MAX_PIXELS: u64 = 120_000_000;
const MAX_PDFS: usize = 50;

fn open(path: &Path) -> AppResult<DynamicImage> {
    let reader = ImageReader::open(path)
        .and_then(|r| r.with_guessed_format())
        .map_err(|_| AppError::InvalidInput("Couldn't open that image".into()))?;
    let (w, h) = reader
        .into_dimensions()
        .map_err(|_| AppError::InvalidInput("That isn't an image Overline can read".into()))?;
    if u64::from(w) * u64::from(h) > MAX_PIXELS {
        return Err(AppError::InvalidInput(
            "That image is too large to convert".into(),
        ));
    }
    ImageReader::open(path)
        .and_then(|r| r.with_guessed_format())
        .map_err(|_| AppError::InvalidInput("Couldn't open that image".into()))?
        .decode()
        .map_err(|_| AppError::InvalidInput("That isn't an image Overline can read".into()))
}

fn save(image: &DynamicImage, path: &Path, target: ImageTarget, quality: u8) -> AppResult<()> {
    let file = std::fs::File::create(path)?;
    let mut out = BufWriter::new(file);
    let result = match target {
        // JPEG has no transparency: flatten onto white rather than black.
        ImageTarget::Jpg => JpegEncoder::new_with_quality(&mut out, quality).encode_image(&flatten(image)),
        ImageTarget::Png => image.write_to(&mut out, ImageFormat::Png),
        ImageTarget::Webp => image.to_rgba8().write_to(&mut out, ImageFormat::WebP),
    };
    result.map_err(|e| {
        let _ = std::fs::remove_file(path);
        AppError::Platform(e.to_string())
    })
}

fn flatten(image: &DynamicImage) -> DynamicImage {
    if !image.color().has_alpha() {
        return DynamicImage::ImageRgb8(image.to_rgb8());
    }
    let rgba = image.to_rgba8();
    let rgb = image::RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let p = rgba.get_pixel(x, y).0;
        let a = u16::from(p[3]);
        let mix = |c: u8| ((u16::from(c) * a + 255 * (255 - a)) / 255) as u8;
        image::Rgb([mix(p[0]), mix(p[1]), mix(p[2])])
    });
    DynamicImage::ImageRgb8(rgb)
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image")
        .to_owned()
}

/// "photo.heic.png"-style names are confusing: "photo.png" (or "photo (2).png").
pub fn convert_image(path: &Path, target: ImageTarget) -> AppResult<PathBuf> {
    let image = open(path)?;
    let out = unique_path(path.with_file_name(format!("{}.{}", stem(path), target.ext())));
    save(&image, &out, target, JPEG_QUALITY)?;
    Ok(out)
}

/// Smaller copy for sharing: half size (at most 2560 px), JPEG unless it needs transparency.
pub fn shrink_image(path: &Path) -> AppResult<PathBuf> {
    let image = open(path)?;
    let (w, h) = (image.width(), image.height());
    let long = w.max(h);
    let target_long = (long / 2).clamp(1, SHRINK_MAX_SIDE).min(long);
    let scale = f64::from(target_long) / f64::from(long);
    let resized = image.resize(
        ((f64::from(w) * scale).round() as u32).max(1),
        ((f64::from(h) * scale).round() as u32).max(1),
        image::imageops::FilterType::Lanczos3,
    );
    let keeps_alpha = resized.color().has_alpha() && has_transparency(&resized);
    let target = if keeps_alpha {
        ImageTarget::Png
    } else {
        ImageTarget::Jpg
    };
    let out = unique_path(path.with_file_name(format!("{} (small).{}", stem(path), target.ext())));
    save(&resized, &out, target, SHRINK_QUALITY)?;
    Ok(out)
}

fn has_transparency(image: &DynamicImage) -> bool {
    image.to_rgba8().pixels().any(|p| p.0[3] < 255)
}

/// Join PDFs in order into "Merged.pdf" next to the first one.
pub fn merge_pdfs(paths: &[PathBuf]) -> AppResult<PathBuf> {
    if paths.len() < 2 || paths.len() > MAX_PDFS {
        return Err(AppError::InvalidInput(
            "Pick between 2 and 50 PDFs to merge".into(),
        ));
    }
    let mut merged = Document::with_version("1.5");
    let mut max_id = 1;
    let mut page_order: Vec<ObjectId> = Vec::new();
    let mut pages = std::collections::BTreeMap::new();
    let mut objects = std::collections::BTreeMap::new();

    for path in paths {
        let mut doc = Document::load(path).map_err(|_| {
            AppError::InvalidInput(format!(
                "Couldn't read {} (it may be password-protected)",
                path.file_name().and_then(|n| n.to_str()).unwrap_or("a PDF")
            ))
        })?;
        if doc.is_encrypted() {
            return Err(AppError::InvalidInput(
                "Password-protected PDFs can't be merged".into(),
            ));
        }
        doc.renumber_objects_with(max_id);
        max_id = doc.max_id + 1;
        // `get_pages` is keyed by page number, so this keeps the reading order.
        for (_, id) in doc.get_pages() {
            if let Ok(object) = doc.get_object(id) {
                page_order.push(id);
                pages.insert(id, object.to_owned());
            }
        }
        objects.extend(doc.objects);
    }

    let mut catalog: Option<(ObjectId, Object)> = None;
    let mut pages_root: Option<(ObjectId, Object)> = None;
    for (id, object) in objects {
        match object.type_name().unwrap_or(b"") {
            b"Catalog" => {
                if catalog.is_none() {
                    catalog = Some((id, object));
                }
            }
            b"Pages" => {
                if pages_root.is_none() {
                    pages_root = Some((id, object));
                }
            }
            b"Page" | b"Outlines" | b"Outline" => {}
            _ => {
                merged.objects.insert(id, object);
            }
        }
    }
    let bad = || AppError::InvalidInput("One of the PDFs is damaged".into());
    let (catalog_id, catalog) = catalog.ok_or_else(bad)?;
    let (pages_id, pages_root) = pages_root.ok_or_else(bad)?;

    for (id, page) in &pages {
        if let Ok(dict) = page.as_dict() {
            let mut dict = dict.clone();
            dict.set("Parent", pages_id);
            merged.objects.insert(*id, Object::Dictionary(dict));
        }
    }
    let mut root = pages_root.as_dict().map_err(|_| bad())?.clone();
    root.set("Count", page_order.len() as u32);
    root.set(
        "Kids",
        page_order.into_iter().map(Object::Reference).collect::<Vec<_>>(),
    );
    merged.objects.insert(pages_id, Object::Dictionary(root));
    let mut catalog = catalog.as_dict().map_err(|_| bad())?.clone();
    catalog.set("Pages", pages_id);
    catalog.remove(b"Outlines");
    merged.objects.insert(catalog_id, Object::Dictionary(catalog));
    merged.trailer.set("Root", catalog_id);
    merged.max_id = merged.objects.keys().map(|(n, _)| *n).max().unwrap_or(1);
    merged.renumber_objects();

    let out = unique_path(paths[0].with_file_name("Merged.pdf"));
    merged.save(&out).map_err(|e| {
        let _ = std::fs::remove_file(&out);
        AppError::Platform(e.to_string())
    })?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("notch-convert-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample_png(dir: &Path, alpha: bool) -> PathBuf {
        let path = dir.join("pic.png");
        let img = image::RgbaImage::from_fn(400, 200, |x, _| {
            image::Rgba([(x % 256) as u8, 80, 160, if alpha && x < 10 { 0 } else { 255 }])
        });
        img.save(&path).unwrap();
        path
    }

    #[test]
    fn converts_and_shrinks_images_without_touching_the_original() {
        let dir = temp_dir("img");
        let png = sample_png(&dir, false);
        let before = std::fs::read(&png).unwrap();

        let jpg = convert_image(&png, ImageTarget::Jpg).unwrap();
        assert_eq!(jpg.file_name().unwrap(), "pic.jpg");
        assert_eq!(image::open(&jpg).unwrap().width(), 400);
        let again = convert_image(&png, ImageTarget::Jpg).unwrap();
        assert_eq!(again.file_name().unwrap(), "pic (2).jpg");
        let webp = convert_image(&png, ImageTarget::Webp).unwrap();
        assert_eq!(image::open(&webp).unwrap().height(), 200);

        let small = shrink_image(&png).unwrap();
        assert_eq!(small.file_name().unwrap(), "pic (small).jpg");
        assert_eq!(image::open(&small).unwrap().width(), 200);
        assert_eq!(std::fs::read(&png).unwrap(), before);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn keeps_transparency_when_shrinking() {
        let dir = temp_dir("alpha");
        let png = sample_png(&dir, true);
        let small = shrink_image(&png).unwrap();
        assert_eq!(small.extension().unwrap(), "png");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn rejects_non_images() {
        let dir = temp_dir("bad");
        let txt = dir.join("note.png");
        std::fs::write(&txt, b"not an image").unwrap();
        assert!(convert_image(&txt, ImageTarget::Jpg).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }

    fn one_page_pdf(path: &Path, pages: usize) {
        use lopdf::{dictionary, Stream};
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let mut kids = Vec::new();
        for i in 0..pages {
            let content = doc.add_object(Stream::new(dictionary! {}, format!("% page {i}").into_bytes()));
            let page = doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Contents" => content,
                "MediaBox" => vec![0.into(), 0.into(), 200.into(), 200.into()],
            });
            kids.push(Object::Reference(page));
        }
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => pages as u32 }),
        );
        let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", catalog);
        doc.save(path).unwrap();
    }

    #[test]
    fn merges_pdfs_in_order() {
        let dir = temp_dir("pdf");
        let a = dir.join("a.pdf");
        let b = dir.join("b.pdf");
        one_page_pdf(&a, 2);
        one_page_pdf(&b, 3);
        let out = merge_pdfs(&[a.clone(), b]).unwrap();
        assert_eq!(out.file_name().unwrap(), "Merged.pdf");
        let merged = Document::load(&out).unwrap();
        assert_eq!(merged.get_pages().len(), 5);
        assert!(merge_pdfs(&[a]).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }
}

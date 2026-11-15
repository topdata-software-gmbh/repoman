//! Bounded image previews: header-only dimensions, size/dimension caps, and
//! explicit `too-large` / `corrupt` / `unsupported` / `missing` states (FR-010).

use super::limits::IMAGE_MAX_BYTES;
use super::{ImagePreview, ImageSide, ImageState};
use base64::Engine;

const SUPPORTED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "svg"];

fn extension(path: &str) -> String {
    path.rsplit('.').next().unwrap_or("").to_ascii_lowercase()
}

pub fn is_image_extension(path: &str) -> bool {
    SUPPORTED_EXTENSIONS.contains(&extension(path).as_str())
}

fn is_svg(path: &str) -> bool {
    extension(path) == "svg"
}

pub fn mime_for(path: &str) -> &'static str {
    match extension(path).as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

/// Builds both sides, or a notice state with no data URL. A side that is absent
/// (added/deleted file) stays `None`.
pub fn build_preview(path: &str, old: Option<&[u8]>, new: Option<&[u8]>) -> ImagePreview {
    if old.is_none() && new.is_none() {
        return ImagePreview {
            state: ImageState::Missing,
            old: None,
            new: None,
        };
    }

    let old_side = old.map(|bytes| encode_side(path, bytes));
    let new_side = new.map(|bytes| encode_side(path, bytes));

    for result in [&old_side, &new_side] {
        if let Some(Err(state)) = result {
            return ImagePreview {
                state: *state,
                old: None,
                new: None,
            };
        }
    }

    ImagePreview {
        state: ImageState::Ready,
        old: old_side.and_then(Result::ok),
        new: new_side.and_then(Result::ok),
    }
}

fn encode_side(path: &str, bytes: &[u8]) -> Result<ImageSide, ImageState> {
    if bytes.len() > IMAGE_MAX_BYTES {
        return Err(ImageState::TooLarge);
    }

    let (width, height) = dimensions(path, bytes)?;
    if super::limits::image_exceeds(bytes.len(), width, height) {
        return Err(ImageState::TooLarge);
    }

    Ok(ImageSide {
        data_url: format!(
            "data:{};base64,{}",
            mime_for(path),
            base64::engine::general_purpose::STANDARD.encode(bytes)
        ),
        width,
        height,
        bytes: bytes.len(),
    })
}

fn dimensions(path: &str, bytes: &[u8]) -> Result<(usize, usize), ImageState> {
    if !is_image_extension(path) {
        return Err(ImageState::Unsupported);
    }
    if is_svg(path) {
        return svg_dimensions(bytes).ok_or(ImageState::Corrupt);
    }
    imagesize::blob_size(bytes)
        .map(|size| (size.width, size.height))
        .map_err(|_| ImageState::Corrupt)
}

fn svg_dimensions(bytes: &[u8]) -> Option<(usize, usize)> {
    let text = String::from_utf8_lossy(bytes);
    let start = text.find("<svg")?;
    let rest = &text[start..];
    let end = rest.find('>')?;
    let tag = &rest[..end];

    let width = attr(tag, "width").and_then(|value| px_value(&value));
    let height = attr(tag, "height").and_then(|value| px_value(&value));
    if let (Some(width), Some(height)) = (width, height) {
        if width > 0 && height > 0 {
            return Some((width, height));
        }
    }

    let view = attr(tag, "viewBox").or_else(|| attr(tag, "viewbox"))?;
    let numbers: Vec<f64> = view
        .split(|character: char| character.is_whitespace() || character == ',')
        .filter_map(|part| part.parse().ok())
        .collect();
    if numbers.len() == 4 && numbers[2] > 0.0 && numbers[3] > 0.0 {
        Some((numbers[2].round() as usize, numbers[3].round() as usize))
    } else {
        None
    }
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let mut from = 0;
    while let Some(offset) = tag[from..].find(name) {
        let absolute = from + offset;
        let before_ok = absolute == 0 || !tag.as_bytes()[absolute - 1].is_ascii_alphanumeric();
        let after = tag[absolute + name.len()..].trim_start();
        if before_ok && after.starts_with('=') {
            let rest = after[1..].trim_start();
            let quote = rest.chars().next()?;
            if quote == '"' || quote == '\'' {
                let body = &rest[1..];
                let end = body.find(quote)?;
                return Some(body[..end].to_string());
            }
        }
        from = absolute + name.len();
        if from >= tag.len() {
            break;
        }
    }
    None
}

fn px_value(raw: &str) -> Option<usize> {
    let trimmed = raw
        .trim()
        .trim_end_matches("px")
        .trim_end_matches("pt")
        .trim();
    let value: f64 = trimmed.parse().ok()?;
    if value.is_finite() && value > 0.0 {
        Some(value.round() as usize)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        // Minimal PNG signature + IHDR (dimensions at bytes 16..24).
        let mut bytes = vec![
            0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13, b'I', b'H', b'D', b'R',
        ];
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&[8, 6, 0, 0, 0]);
        bytes.extend_from_slice(&[0, 0, 0, 0]);
        bytes
    }

    #[test]
    fn reads_png_dimensions_and_encodes_a_data_url() {
        let preview = build_preview("logo.png", None, Some(&png(80, 40)));
        assert_eq!(preview.state, ImageState::Ready);
        assert!(preview.old.is_none());
        let new = preview.new.expect("new side");
        assert_eq!((new.width, new.height), (80, 40));
        assert!(new.data_url.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn parses_svg_from_width_and_viewbox() {
        let with_attrs = br#"<svg width="120" height="30"><rect/></svg>"#;
        assert_eq!(svg_dimensions(with_attrs), Some((120, 30)));

        let with_viewbox = br#"<svg viewBox="0 0 64 48"><rect/></svg>"#;
        assert_eq!(svg_dimensions(with_viewbox), Some((64, 48)));
    }

    #[test]
    fn corrupt_and_oversized_images_become_notice_states() {
        let corrupt = build_preview("broken.png", None, Some(b"not an image"));
        assert_eq!(corrupt.state, ImageState::Corrupt);
        assert!(corrupt.new.is_none());

        let oversized = vec![0u8; IMAGE_MAX_BYTES + 1];
        let preview = build_preview("huge.png", None, Some(&oversized));
        assert_eq!(preview.state, ImageState::TooLarge);
        assert!(preview.new.is_none());
    }

    #[test]
    fn missing_sides_stay_null() {
        let preview = build_preview("logo.png", None, None);
        assert_eq!(preview.state, ImageState::Missing);
        assert!(preview.old.is_none() && preview.new.is_none());
    }
}

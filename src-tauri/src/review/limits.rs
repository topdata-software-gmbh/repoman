//! Preview bounds shared by the review core (data-model E4, contract C5).

use serde::{Deserialize, Serialize};

/// 2 MiB of preview text.
pub const TEXT_MAX_BYTES: usize = 2_097_152;
/// 5,000 diff rows per preview.
pub const TEXT_MAX_DIFF_LINES: usize = 5_000;
/// 10 MiB per image side.
pub const IMAGE_MAX_BYTES: usize = 10_485_760;
/// 8,000 px in either dimension.
pub const IMAGE_MAX_DIMENSION: usize = 8_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TruncationReason {
    Bytes,
    Lines,
}

/// The first cap that was hit, if any; `bytes` wins ties because the text was
/// clipped before rows were produced.
pub fn truncation_reason(bytes: usize, diff_lines: usize) -> Option<TruncationReason> {
    if bytes > TEXT_MAX_BYTES {
        Some(TruncationReason::Bytes)
    } else if diff_lines > TEXT_MAX_DIFF_LINES {
        Some(TruncationReason::Lines)
    } else {
        None
    }
}

pub fn image_exceeds(bytes: usize, width: usize, height: usize) -> bool {
    bytes > IMAGE_MAX_BYTES || width > IMAGE_MAX_DIMENSION || height > IMAGE_MAX_DIMENSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_caps_stop_at_the_first_limit() {
        assert_eq!(truncation_reason(TEXT_MAX_BYTES, 0), None);
        assert_eq!(
            truncation_reason(TEXT_MAX_BYTES + 1, TEXT_MAX_DIFF_LINES + 1),
            Some(TruncationReason::Bytes)
        );
        assert_eq!(
            truncation_reason(TEXT_MAX_BYTES, TEXT_MAX_DIFF_LINES + 1),
            Some(TruncationReason::Lines)
        );
    }

    #[test]
    fn image_bounds_cover_bytes_and_dimensions() {
        assert!(!image_exceeds(
            IMAGE_MAX_BYTES,
            IMAGE_MAX_DIMENSION,
            IMAGE_MAX_DIMENSION
        ));
        assert!(image_exceeds(IMAGE_MAX_BYTES + 1, 10, 10));
        assert!(image_exceeds(10, IMAGE_MAX_DIMENSION + 1, 10));
        assert!(image_exceeds(10, 10, IMAGE_MAX_DIMENSION + 1));
    }
}

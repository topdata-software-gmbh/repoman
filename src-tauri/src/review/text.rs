//! `git diff --unified=0` parsing into side-by-side rows and fold groups
//! (data-model E5, research R5).

use super::limits::{truncation_reason, TruncationReason, TEXT_MAX_BYTES, TEXT_MAX_DIFF_LINES};
use super::{DiffLine, TextPreview};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Hunk {
    old_start: usize,
    old_count: usize,
    new_start: usize,
    new_count: usize,
}

fn parse_range(spec: &str) -> (usize, usize) {
    match spec.split_once(',') {
        Some((start, count)) => (start.parse().unwrap_or(0), count.parse().unwrap_or(0)),
        None => (spec.parse().unwrap_or(0), 1),
    }
}

fn parse_header(line: &str) -> Option<Hunk> {
    let rest = line.strip_prefix("@@ ")?;
    let end = rest.find(" @@")?;
    let mut parts = rest[..end].split(' ');
    let old = parts.next()?.strip_prefix('-')?;
    let new = parts.next()?.strip_prefix('+')?;
    let (old_start, old_count) = parse_range(old);
    let (new_start, new_count) = parse_range(new);
    Some(Hunk {
        old_start,
        old_count,
        new_start,
        new_count,
    })
}

fn parse_hunks(diff_text: &str) -> Vec<Hunk> {
    diff_text.lines().filter_map(parse_header).collect()
}

fn clip(text: &str, max: usize) -> (String, bool) {
    if text.len() <= max {
        return (text.to_string(), false);
    }
    let mut end = max;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    (text[..end].to_string(), true)
}

fn lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut parts: Vec<&str> = text.split('\n').collect();
    if parts.last() == Some(&"") {
        parts.pop();
    }
    parts
        .into_iter()
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect()
}

fn line_at(lines: &[&str], index: usize) -> String {
    lines.get(index).copied().unwrap_or("").to_string()
}

fn same(old_number: usize, new_number: usize, text: &str) -> DiffLine {
    DiffLine::Same {
        old_number,
        new_number,
        old_text: text.to_string(),
        new_text: text.to_string(),
    }
}

fn push_fold(
    rows: &mut Vec<DiffLine>,
    old_start: usize,
    new_start: usize,
    count: usize,
    old_lines: &[&str],
) {
    if count == 0 {
        return;
    }
    let inner = (0..count)
        .map(|offset| {
            same(
                old_start + offset + 1,
                new_start + offset + 1,
                old_lines.get(old_start + offset).copied().unwrap_or(""),
            )
        })
        .collect();
    rows.push(DiffLine::Fold {
        old_start: old_start + 1,
        new_start: new_start + 1,
        count,
        rows: inner,
    });
}

/// Builds the friendly rows and the optional raw unified text from full old/new
/// sides plus `git diff --unified=0` output.
pub fn build_preview(old_text: &str, new_text: &str, diff_text: &str) -> TextPreview {
    let total = old_text.len().saturating_add(new_text.len());
    let (old_owned, new_owned, bytes_truncated) = if total > TEXT_MAX_BYTES {
        let half = TEXT_MAX_BYTES / 2;
        let (old, _) = clip(old_text, half);
        let (new, _) = clip(new_text, half);
        (old, new, true)
    } else {
        (old_text.to_string(), new_text.to_string(), false)
    };

    let old_lines = lines(&old_owned);
    let new_lines = lines(&new_owned);
    let hunks = parse_hunks(diff_text);
    let mut rows: Vec<DiffLine> = Vec::new();

    if hunks.is_empty() {
        if old_owned == new_owned {
            push_fold(&mut rows, 0, 0, old_lines.len(), &old_lines);
        } else {
            let paired = old_lines.len().min(new_lines.len());
            for index in 0..paired {
                rows.push(DiffLine::Changed {
                    old_number: index + 1,
                    new_number: index + 1,
                    old_text: line_at(&old_lines, index),
                    new_text: line_at(&new_lines, index),
                });
            }
            for index in paired..old_lines.len() {
                rows.push(DiffLine::Removed {
                    old_number: index + 1,
                    old_text: line_at(&old_lines, index),
                });
            }
            for index in paired..new_lines.len() {
                rows.push(DiffLine::Added {
                    new_number: index + 1,
                    new_text: line_at(&new_lines, index),
                });
            }
        }
    } else {
        let mut old_pos = 0usize;
        let mut new_pos = 0usize;

        for hunk in hunks {
            let old_start = if hunk.old_count == 0 {
                hunk.old_start
            } else {
                hunk.old_start.saturating_sub(1)
            };
            let new_start = if hunk.new_count == 0 {
                hunk.new_start
            } else {
                hunk.new_start.saturating_sub(1)
            };

            let gap = old_start
                .saturating_sub(old_pos)
                .min(new_start.saturating_sub(new_pos));
            push_fold(&mut rows, old_pos, new_pos, gap, &old_lines);

            let paired = hunk.old_count.min(hunk.new_count);
            for offset in 0..paired {
                rows.push(DiffLine::Changed {
                    old_number: old_start + offset + 1,
                    new_number: new_start + offset + 1,
                    old_text: line_at(&old_lines, old_start + offset),
                    new_text: line_at(&new_lines, new_start + offset),
                });
            }
            for offset in paired..hunk.old_count {
                rows.push(DiffLine::Removed {
                    old_number: old_start + offset + 1,
                    old_text: line_at(&old_lines, old_start + offset),
                });
            }
            for offset in paired..hunk.new_count {
                rows.push(DiffLine::Added {
                    new_number: new_start + offset + 1,
                    new_text: line_at(&new_lines, new_start + offset),
                });
            }

            old_pos = old_start + hunk.old_count;
            new_pos = new_start + hunk.new_count;
        }

        let old_remaining = old_lines.len().saturating_sub(old_pos);
        let new_remaining = new_lines.len().saturating_sub(new_pos);
        let trailing = old_remaining.min(new_remaining);
        push_fold(&mut rows, old_pos, new_pos, trailing, &old_lines);
        for offset in trailing..old_remaining {
            rows.push(DiffLine::Removed {
                old_number: old_pos + offset + 1,
                old_text: line_at(&old_lines, old_pos + offset),
            });
        }
        for offset in trailing..new_remaining {
            rows.push(DiffLine::Added {
                new_number: new_pos + offset + 1,
                new_text: line_at(&new_lines, new_pos + offset),
            });
        }
    }

    let reason = truncation_reason(
        if bytes_truncated {
            TEXT_MAX_BYTES + 1
        } else {
            total
        },
        rows.len(),
    );
    if reason == Some(TruncationReason::Lines) {
        rows.truncate(TEXT_MAX_DIFF_LINES);
    }

    let raw = if reason.is_none() && !diff_text.is_empty() && diff_text.len() <= TEXT_MAX_BYTES {
        Some(diff_text.to_string())
    } else {
        None
    };

    TextPreview {
        rows,
        truncated: reason.is_some(),
        truncation_reason: reason,
        raw,
        language: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::status::{classify_kind, NumStat};
    use crate::review::DiffKind;

    fn kinds(rows: &[DiffLine]) -> Vec<&'static str> {
        rows.iter()
            .map(|row| match row {
                DiffLine::Same { .. } => "same",
                DiffLine::Changed { .. } => "changed",
                DiffLine::Added { .. } => "added",
                DiffLine::Removed { .. } => "removed",
                DiffLine::Fold { .. } => "fold",
            })
            .collect()
    }

    #[test]
    fn builds_changed_rows_folds_and_numbers() {
        let old = "a\nb\nc\nd\n";
        let new = "a\nB\nc\nd\n";
        let diff = "@@ -2,1 +2,1 @@\n-b\n+B\n";
        let preview = build_preview(old, new, diff);

        assert_eq!(kinds(&preview.rows), vec!["fold", "changed", "fold"]);
        match &preview.rows[1] {
            DiffLine::Changed {
                old_number,
                new_number,
                old_text,
                new_text,
            } => {
                assert_eq!((*old_number, *new_number), (2, 2));
                assert_eq!(old_text, "b");
                assert_eq!(new_text, "B");
            }
            other => panic!("expected changed row, got {other:?}"),
        }
    }

    #[test]
    fn added_and_removed_sides_have_one_null_side() {
        let added = build_preview("", "x\ny\n", "");
        assert_eq!(kinds(&added.rows), vec!["added", "added"]);
        let removed = build_preview("x\ny\n", "", "");
        assert_eq!(kinds(&removed.rows), vec!["removed", "removed"]);
    }

    #[test]
    fn truncation_reports_lines_and_omits_raw() {
        let old: String = (0..6000).map(|i| format!("line {i}\n")).collect();
        let new: String = (0..6000).map(|i| format!("changed {i}\n")).collect();
        let preview = build_preview(&old, &new, "@@ -1,6000 +1,6000 @@\n");
        assert!(preview.truncated);
        assert_eq!(preview.truncation_reason, Some(TruncationReason::Lines));
        assert_eq!(preview.rows.len(), TEXT_MAX_DIFF_LINES);
        assert!(preview.raw.is_none());
    }

    #[test]
    fn nul_bytes_classify_as_binary() {
        assert_eq!(
            classify_kind("image.dat", None, b"abc\0def"),
            DiffKind::Binary
        );
        assert_eq!(
            classify_kind(
                "image.dat",
                Some(&NumStat {
                    added: 0,
                    removed: 0,
                    binary: true
                }),
                b"abc"
            ),
            DiffKind::Binary
        );
    }
}

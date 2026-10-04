//! Label generator and ergonomic target assigner.
//!
//! Assigns conflict-free, prefix-free home-row jump badges to navigation targets
//! prioritizing proximity to the current cursor position.

/// Home-row prioritized alphabet optimized for touch-typing ergonomics.
pub const ALPHABET: &[char] = &[
    'a', 's', 'd', 'f', 'j', 'k', 'l', ';', 'g', 'h', 'q', 'w', 'e', 'r', 'u', 'i', 'o', 'p',
];

/// A navigational target located at a specific screen coordinate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchTarget {
    /// 0-indexed vertical screen or buffer row.
    pub row: usize,
    /// 0-indexed horizontal column index.
    pub col: usize,
    /// Identifier of the tmux pane where this target resides.
    pub pane_id: String,
    /// Matched text snippet corresponding to this target.
    pub matched_text: String,
}

/// A target annotated with an ergonomic jump label badge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabeledTarget {
    /// The underlying navigational target.
    pub target: MatchTarget,
    /// The unique jump label string (e.g. `"a"`, `"s"`, `"pa"`).
    pub label: String,
}

/// Calculates Manhattan distance between a target and the cursor.
#[inline]
fn distance_to_cursor(target: &MatchTarget, cursor_row: usize, cursor_col: usize) -> usize {
    let row_diff = (target.row as isize - cursor_row as isize).unsigned_abs();
    let col_diff = (target.col as isize - cursor_col as isize).unsigned_abs();
    row_diff + col_diff
}

/// Generates a set of unique, prefix-free labels for `count` targets.
///
/// Uses 1-character labels when `count <= ALPHABET.len()`, and dynamically expands
/// a 2-character prefix tree when `count > ALPHABET.len()`.
pub fn generate_labels(count: usize) -> Vec<String> {
    if count == 0 {
        return Vec::new();
    }

    let m = ALPHABET.len();
    if count <= m {
        return ALPHABET.iter().take(count).map(|c| c.to_string()).collect();
    }

    // When count > m, allocate prefix slots from the end of the alphabet.
    // Each prefix slot expands 1 single-char slot into m two-char slots (net gain: m - 1).
    let max_2char = m * m;
    if count <= max_2char {
        // Ceiling division: (count - m + (m - 2)) / (m - 1)
        let p = (count - m + (m - 2)) / (m - 1);
        let k = m - p;
        let mut labels = Vec::with_capacity(count);

        // Single-character leaves for highest-ergonomics keys
        for &c in &ALPHABET[..k] {
            labels.push(c.to_string());
        }

        // Two-character leaves using prefixes from the least ergonomic keys
        for &pfx in &ALPHABET[k..m] {
            for &sfx in ALPHABET {
                labels.push(format!("{pfx}{sfx}"));
                if labels.len() == count {
                    return labels;
                }
            }
        }
        return labels;
    }

    // Fallback for > 324 targets (3-char labels)
    let mut labels = Vec::with_capacity(count);
    for &c1 in ALPHABET {
        for &c2 in ALPHABET {
            for &c3 in ALPHABET {
                labels.push(format!("{c1}{c2}{c3}"));
                if labels.len() == count {
                    return labels;
                }
            }
        }
    }
    labels
}

/// Assigns unique, conflict-free jump labels to `targets`, sorted by distance to the cursor.
///
/// Targets closest to `(cursor_row, cursor_col)` receive the most ergonomic single-character
/// home-row labels. If the number of targets exceeds the alphabet length, a prefix tree is
/// formed so that closer targets retain single-character labels while farther targets receive
/// two-character labels without prefix conflicts.
pub fn assign_labels(
    targets: &[MatchTarget],
    cursor_row: usize,
    cursor_col: usize,
) -> Vec<LabeledTarget> {
    if targets.is_empty() {
        return Vec::new();
    }

    let mut sorted_targets = targets.to_vec();
    sorted_targets.sort_by(|a, b| {
        let dist_a = distance_to_cursor(a, cursor_row, cursor_col);
        let dist_b = distance_to_cursor(b, cursor_row, cursor_col);
        dist_a
            .cmp(&dist_b)
            .then_with(|| a.row.cmp(&b.row))
            .then_with(|| a.col.cmp(&b.col))
    });

    let labels = generate_labels(sorted_targets.len());
    sorted_targets
        .into_iter()
        .zip(labels)
        .map(|(target, label)| LabeledTarget { target, label })
        .collect()
}

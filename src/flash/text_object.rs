//! Semantic text objects and remote target resolution.
//!
//! Provides text object extraction (words, quoted literals, balanced brackets,
//! lines, and auto-detected semantic tokens) for Flash jump and remote operator workflows.

/// Target semantic text object type for remote operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextObject {
    /// Semantic token recognized by the extraction scanner, falling back to [`TextObject::InnerWord`].
    AutoToken,
    /// Contiguous alphanumeric and underscore sequence.
    InnerWord,
    /// Word plus surrounding whitespace (trailing preferred, leading if at line end).
    AWord,
    /// Content strictly inside quotes (`"..."`, `'...'`, or `` `...` ``).
    InnerQuoted,
    /// Quoted content including bounding quote characters.
    AQuoted,
    /// Entire line trimmed of leading and trailing whitespace.
    Line,
    /// Substring from target column to end of line.
    ToEndOfLine,
    /// Balanced bracket pair including delimiters (`(...)`, `[...]`, `{...}`).
    Bracket,
}

/// Resolves the character span `(col_start, col_end, text)` of a text object at column `col` in `line`.
///
/// Both `col_start` and `col_end` are 0-indexed character offsets (half-open interval `[col_start, col_end)`).
pub fn resolve_text_object_span(
    line: &str,
    col: usize,
    text_object: TextObject,
) -> Option<(usize, usize, String)> {
    if text_object == TextObject::AutoToken {
        return resolve_auto_token_span(line, col);
    }

    let chars: Vec<(usize, char)> = line.char_indices().collect();
    match text_object {
        TextObject::AutoToken => unreachable!(),
        TextObject::InnerWord => resolve_inner_word_span(&chars, line, col),
        TextObject::AWord => resolve_aword_span(&chars, line, col),
        TextObject::InnerQuoted => resolve_quoted_span(&chars, line, col, false),
        TextObject::AQuoted => resolve_quoted_span(&chars, line, col, true),
        TextObject::Line => resolve_line_span(&chars, line, col),
        TextObject::ToEndOfLine => resolve_to_end_of_line_span(&chars, line, col),
        TextObject::Bracket => resolve_bracket_span(&chars, line, col),
    }
}

/// Resolves the string content of a text object at column `col` in `line`.
#[inline]
pub fn resolve_text_object(line: &str, col: usize, text_object: TextObject) -> Option<String> {
    resolve_text_object_span(line, col, text_object).map(|(_, _, text)| text)
}

#[inline]
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn char_span_to_byte_span(
    chars: &[(usize, char)],
    line: &str,
    start_char: usize,
    end_char: usize,
) -> (usize, usize) {
    let byte_start = if start_char < chars.len() {
        chars[start_char].0
    } else {
        line.len()
    };
    let byte_end = if end_char < chars.len() {
        chars[end_char].0
    } else {
        line.len()
    };
    (byte_start, byte_end)
}

fn find_inner_word(chars: &[(usize, char)], col: usize) -> Option<(usize, usize)> {
    if col >= chars.len() || !is_word_char(chars[col].1) {
        return None;
    }

    let mut start = col;
    while start > 0 && is_word_char(chars[start - 1].1) {
        start -= 1;
    }

    let mut end = col + 1;
    while end < chars.len() && is_word_char(chars[end].1) {
        end += 1;
    }

    Some((start, end))
}

fn resolve_inner_word_span(
    chars: &[(usize, char)],
    line: &str,
    col: usize,
) -> Option<(usize, usize, String)> {
    let (start, end) = find_inner_word(chars, col)?;
    let (byte_start, byte_end) = char_span_to_byte_span(chars, line, start, end);
    Some((start, end, line[byte_start..byte_end].to_string()))
}

fn resolve_aword_span(
    chars: &[(usize, char)],
    line: &str,
    col: usize,
) -> Option<(usize, usize, String)> {
    let (word_start, word_end) = find_inner_word(chars, col)?;

    let mut end = word_end;
    while end < chars.len() && chars[end].1.is_whitespace() {
        end += 1;
    }

    let (start, end) = if end > word_end {
        (word_start, end)
    } else {
        let mut start = word_start;
        while start > 0 && chars[start - 1].1.is_whitespace() {
            start -= 1;
        }
        (start, word_end)
    };

    let (byte_start, byte_end) = char_span_to_byte_span(chars, line, start, end);
    Some((start, end, line[byte_start..byte_end].to_string()))
}

fn find_quoted_pairs(chars: &[(usize, char)], delim: char) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].1 == delim {
            if delim == '\'' && i > 0 && chars[i - 1].1.is_alphanumeric() {
                i += 1;
                continue;
            }

            let mut j = i + 1;
            let mut found = false;
            while j < chars.len() {
                if chars[j].1 == '\\' {
                    j += 2;
                    continue;
                }
                if chars[j].1 == delim {
                    if delim == '\'' && j + 1 < chars.len() && chars[j + 1].1.is_alphanumeric() {
                        j += 1;
                        continue;
                    }
                    found = true;
                    break;
                }
                j += 1;
            }

            if found {
                pairs.push((i, j));
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    pairs
}

fn find_all_quoted_pairs(chars: &[(usize, char)]) -> Vec<(usize, usize)> {
    let mut all_pairs = Vec::new();
    for delim in ['"', '\'', '`'] {
        all_pairs.extend(find_quoted_pairs(chars, delim));
    }
    all_pairs
}

fn resolve_quoted_span(
    chars: &[(usize, char)],
    line: &str,
    col: usize,
    include_delimiters: bool,
) -> Option<(usize, usize, String)> {
    if chars.is_empty() || col > chars.len() {
        return None;
    }

    let pairs = find_all_quoted_pairs(chars);
    let mut matching: Vec<_> = pairs
        .into_iter()
        .filter(|&(open, close)| open <= col && col <= close)
        .collect();

    if matching.is_empty() {
        return None;
    }

    matching.sort_by_key(|&(open, close)| close - open);
    let (open_idx, close_idx) = matching[0];

    let (start, end) = if include_delimiters {
        (open_idx, close_idx + 1)
    } else {
        (open_idx + 1, close_idx)
    };

    let (byte_start, byte_end) = char_span_to_byte_span(chars, line, start, end);
    let text = line[byte_start..byte_end].to_string();
    Some((start, end, text))
}

fn find_all_bracket_pairs(chars: &[(usize, char)]) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    let mut stack: Vec<(char, usize)> = Vec::new();

    for (idx, &(_, c)) in chars.iter().enumerate() {
        match c {
            '(' | '[' | '{' => {
                stack.push((c, idx));
            }
            ')' => {
                if let Some(pos) = stack.iter().rposition(|&(open_c, _)| open_c == '(') {
                    let (_, open_idx) = stack[pos];
                    stack.truncate(pos);
                    pairs.push((open_idx, idx));
                }
            }
            ']' => {
                if let Some(pos) = stack.iter().rposition(|&(open_c, _)| open_c == '[') {
                    let (_, open_idx) = stack[pos];
                    stack.truncate(pos);
                    pairs.push((open_idx, idx));
                }
            }
            '}' => {
                if let Some(pos) = stack.iter().rposition(|&(open_c, _)| open_c == '{') {
                    let (_, open_idx) = stack[pos];
                    stack.truncate(pos);
                    pairs.push((open_idx, idx));
                }
            }
            _ => {}
        }
    }
    pairs
}

fn resolve_bracket_span(
    chars: &[(usize, char)],
    line: &str,
    col: usize,
) -> Option<(usize, usize, String)> {
    if chars.is_empty() || col > chars.len() {
        return None;
    }

    let pairs = find_all_bracket_pairs(chars);
    let mut matching: Vec<_> = pairs
        .into_iter()
        .filter(|&(open, close)| open <= col && col <= close)
        .collect();

    if matching.is_empty() {
        return None;
    }

    matching.sort_by_key(|&(open, close)| close - open);
    let (open_idx, close_idx) = matching[0];

    let start = open_idx;
    let end = close_idx + 1;
    let (byte_start, byte_end) = char_span_to_byte_span(chars, line, start, end);
    let text = line[byte_start..byte_end].to_string();
    Some((start, end, text))
}

fn resolve_line_span(
    chars: &[(usize, char)],
    line: &str,
    _col: usize,
) -> Option<(usize, usize, String)> {
    if line.trim().is_empty() {
        return Some((0, chars.len(), line.to_string()));
    }

    let first_non_ws = chars.iter().position(|(_, c)| !c.is_whitespace())?;
    let last_non_ws = chars.iter().rposition(|(_, c)| !c.is_whitespace())?;

    let start = first_non_ws;
    let end = last_non_ws + 1;
    let (byte_start, byte_end) = char_span_to_byte_span(chars, line, start, end);
    let text = line[byte_start..byte_end].to_string();
    Some((start, end, text))
}

fn resolve_to_end_of_line_span(
    chars: &[(usize, char)],
    line: &str,
    col: usize,
) -> Option<(usize, usize, String)> {
    if col > chars.len() {
        return None;
    }

    let start = col;
    let end = chars.len();
    let (byte_start, byte_end) = char_span_to_byte_span(chars, line, start, end);
    let text = line[byte_start..byte_end].to_string();
    Some((start, end, text))
}

fn resolve_auto_token_span(line: &str, col: usize) -> Option<(usize, usize, String)> {
    let tokens = crate::extract::scanner::scan_line(line, 0, "");
    let mut matching: Vec<_> = tokens
        .into_iter()
        .filter(|t| t.col_start <= col && col < t.col_end)
        .collect();

    if !matching.is_empty() {
        matching.sort_by_key(|t| t.col_end - t.col_start);
        let best = matching.remove(0);
        return Some((best.col_start, best.col_end, best.clean_text));
    }

    let chars: Vec<(usize, char)> = line.char_indices().collect();
    resolve_inner_word_span(&chars, line, col)
}

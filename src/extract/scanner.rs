//! Fast token scanner and entity extraction pipeline.
//!
//! Provides zero-external-dependency extraction of paths, URLs, git hashes,
//! IP addresses, and quoted literals from terminal output and scrollback lines.

use crate::extract::model::{EntityKind, ExtractedToken};

/// Scans a single line of terminal output and extracts all recognizable entities.
///
/// Tokens are returned in order of appearance (`col_start`, then `col_end`).
pub fn scan_line(line: &str, screen_row: usize, pane_id: &str) -> Vec<ExtractedToken> {
    let mut tokens = Vec::new();
    let mut occupied_ranges = Vec::new();

    // 1. Quoted literals (can span whitespace)
    extract_quoted(line, screen_row, pane_id, &mut tokens);

    // 2. URLs (takes precedence over paths, hashes, and IPs)
    extract_urls(line, screen_row, pane_id, &mut tokens, &mut occupied_ranges);

    // 3. IPv4 addresses
    extract_ips(line, screen_row, pane_id, &occupied_ranges, &mut tokens);

    // 4. File paths with slashes
    extract_paths(line, screen_row, pane_id, &occupied_ranges, &mut tokens);

    // 5. Git commit hashes / hex identifiers
    extract_hashes(line, screen_row, pane_id, &occupied_ranges, &mut tokens);

    // Sort tokens by screen column order for natural navigation hints
    tokens.sort_by_key(|t| (t.col_start, t.col_end));
    tokens
        .dedup_by(|a, b| a.kind == b.kind && a.col_start == b.col_start && a.col_end == b.col_end);

    tokens
}

/// Scans multiple lines of terminal output and extracts all recognizable entities.
pub fn scan_lines(lines: &[String], pane_id: &str) -> Vec<ExtractedToken> {
    let mut all_tokens = Vec::new();
    for (row_idx, line) in lines.iter().enumerate() {
        all_tokens.extend(scan_line(line, row_idx, pane_id));
    }
    all_tokens
}

/// Iterator yielding byte offsets and slices for whitespace-delimited words.
struct WordsWithOffsets<'a> {
    s: &'a str,
    byte_offset: usize,
}

impl<'a> WordsWithOffsets<'a> {
    fn new(s: &'a str) -> Self {
        Self { s, byte_offset: 0 }
    }
}

impl<'a> Iterator for WordsWithOffsets<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        if self.byte_offset >= self.s.len() {
            return None;
        }
        let remainder = &self.s[self.byte_offset..];
        let trimmed_start = remainder.trim_start();
        if trimmed_start.is_empty() {
            self.byte_offset = self.s.len();
            return None;
        }
        let start_offset = self.s.len() - trimmed_start.len();
        let word_len = trimmed_start
            .find(|c: char| c.is_whitespace())
            .unwrap_or(trimmed_start.len());
        let word = &trimmed_start[..word_len];
        self.byte_offset = start_offset + word_len;
        Some((start_offset, word))
    }
}

/// Checks whether a candidate byte range `[start, end)` overlaps any existing range in `ranges`.
fn is_overlapping(start: usize, end: usize, ranges: &[(usize, usize)]) -> bool {
    ranges
        .iter()
        .any(|&(r_start, r_end)| start < r_end && end > r_start)
}

/// Extracts strings enclosed in `"..."`, `'...'`, or `` `...` ``.
fn extract_quoted(line: &str, screen_row: usize, pane_id: &str, tokens: &mut Vec<ExtractedToken>) {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let quote = bytes[i];
        if quote == b'"' || quote == b'`' || quote == b'\'' {
            // Avoid misidentifying apostrophes in English words (e.g. don't, it's)
            if quote == b'\''
                && i > 0
                && line[..i]
                    .chars()
                    .next_back()
                    .is_some_and(|prev| prev.is_alphanumeric())
            {
                i += 1;
                continue;
            }

            // Search for closing matching quote, respecting backslash escapes
            let mut j = i + 1;
            let mut found = false;
            while j < bytes.len() {
                if bytes[j] == b'\\' {
                    j += 2;
                    continue;
                }
                if bytes[j] == quote {
                    found = true;
                    break;
                }
                j += 1;
            }

            if found && j > i + 1 {
                // Ensure single quote does not end adjacent to an alphanumeric word
                let valid_quote = if quote == b'\'' && j + 1 < bytes.len() {
                    if let Some(next_ch) = line[j + 1..].chars().next() {
                        !next_ch.is_alphanumeric()
                    } else {
                        true
                    }
                } else {
                    true
                };

                if valid_quote {
                    let raw_slice = &line[i..=j];
                    let clean_slice = &line[i + 1..j];
                    let col_start = line[..i].chars().count();
                    let col_end = col_start + raw_slice.chars().count();

                    tokens.push(ExtractedToken {
                        kind: EntityKind::Quoted,
                        raw_text: raw_slice.to_string(),
                        clean_text: clean_slice.to_string(),
                        line_number: None,
                        col_number: None,
                        pane_id: pane_id.to_string(),
                        screen_row,
                        col_start,
                        col_end,
                    });

                    i = j + 1;
                    continue;
                }
            }
        }
        i += 1;
    }
}

/// Extracts URLs starting with `https://`, `http://`, or `git@`.
fn extract_urls(
    line: &str,
    screen_row: usize,
    pane_id: &str,
    tokens: &mut Vec<ExtractedToken>,
    occupied_ranges: &mut Vec<(usize, usize)>,
) {
    const URL_SCHEMES: [&str; 3] = ["https://", "http://", "git@"];

    for scheme in &URL_SCHEMES {
        let mut search_from = 0;
        while search_from < line.len() {
            let Some(pos) = line[search_from..].find(scheme) else {
                break;
            };
            let start = search_from + pos;

            // Ensure not preceded by alphanumeric (avoid false positives in identifiers)
            let valid_start = if start == 0 {
                true
            } else {
                line[..start]
                    .chars()
                    .next_back()
                    .is_none_or(|c| !c.is_alphanumeric())
            };

            let remainder = &line[start..];
            let end_offset = remainder
                .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '`' | '<' | '>'))
                .unwrap_or(remainder.len());

            let mut candidate = &line[start..start + end_offset];

            // Strip trailing punctuation from URL boundary
            while candidate.len() > scheme.len()
                && candidate.chars().next_back().is_some_and(|c| {
                    matches!(
                        c,
                        ',' | '.' | ')' | ';' | ']' | '}' | '>' | ':' | '\'' | '"'
                    )
                })
            {
                let last_len = candidate.chars().next_back().unwrap().len_utf8();
                candidate = &candidate[..candidate.len() - last_len];
            }

            let end = start + candidate.len();
            if valid_start
                && candidate.len() > scheme.len()
                && !is_overlapping(start, end, occupied_ranges)
            {
                let col_start = line[..start].chars().count();
                let col_end = col_start + candidate.chars().count();

                tokens.push(ExtractedToken {
                    kind: EntityKind::Url,
                    raw_text: candidate.to_string(),
                    clean_text: candidate.to_string(),
                    line_number: None,
                    col_number: None,
                    pane_id: pane_id.to_string(),
                    screen_row,
                    col_start,
                    col_end,
                });
                occupied_ranges.push((start, end));
            }

            search_from = start + scheme.len();
        }
    }
}

/// Parses and validates IPv4 address string, optionally with port suffix.
fn parse_ipv4(s: &str) -> bool {
    let (ip_str, port_str) = match s.split_once(':') {
        Some((ip, port)) => {
            // Port must be a valid 1-65535 decimal number
            match port.parse::<u16>() {
                Ok(p) if p > 0 => (ip, Some(port)),
                _ => return false,
            }
        }
        None => (s, None),
    };

    let segments: Vec<&str> = ip_str.split('.').collect();
    if segments.len() != 4 {
        return false;
    }

    for segment in segments {
        if segment.is_empty() || segment.len() > 3 || !segment.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
        // Leading zeros for values > 0 are usually invalid or octal in strict IP
        if segment.len() > 1 && segment.starts_with('0') {
            return false;
        }
        if segment.parse::<u8>().is_err() {
            return false;
        }
    }

    let _ = port_str;
    true
}

/// Extracts IPv4 addresses (with optional ports).
fn extract_ips(
    line: &str,
    screen_row: usize,
    pane_id: &str,
    occupied_ranges: &[(usize, usize)],
    tokens: &mut Vec<ExtractedToken>,
) {
    for (word_start, word) in WordsWithOffsets::new(line) {
        // Strip leading boundary punctuation
        let leading_len = word
            .char_indices()
            .take_while(|(_, c)| matches!(*c, '(' | '[' | '{' | '<' | '"' | '\''))
            .map(|(idx, c)| idx + c.len_utf8())
            .last()
            .unwrap_or(0);

        let trimmed_leading = &word[leading_len..];

        // Strip trailing punctuation
        let mut candidate = trimmed_leading;
        while let Some(last_ch) = candidate.chars().next_back() {
            if matches!(
                last_ch,
                ',' | '.' | ')' | ';' | ']' | '}' | '>' | '\'' | '"'
            ) {
                candidate = &candidate[..candidate.len() - last_ch.len_utf8()];
            } else {
                break;
            }
        }

        if candidate.is_empty() {
            continue;
        }

        let cand_start = word_start + leading_len;
        let cand_end = cand_start + candidate.len();

        if is_overlapping(cand_start, cand_end, occupied_ranges) {
            continue;
        }

        if parse_ipv4(candidate) {
            let col_start = line[..cand_start].chars().count();
            let col_end = col_start + candidate.chars().count();

            tokens.push(ExtractedToken {
                kind: EntityKind::Ip,
                raw_text: candidate.to_string(),
                clean_text: candidate.to_string(),
                line_number: None,
                col_number: None,
                pane_id: pane_id.to_string(),
                screen_row,
                col_start,
                col_end,
            });
        }
    }
}

/// Parses path candidate with optional `:line` and `:col` suffix.
fn parse_path_candidate(candidate: &str) -> Option<(String, Option<usize>, Option<usize>)> {
    // Path must contain at least one slash '/' or start with '~/'
    if !candidate.contains('/') && !candidate.starts_with("~/") {
        return None;
    }

    // Must not be a URL scheme
    if candidate.starts_with("http://")
        || candidate.starts_with("https://")
        || candidate.starts_with("git@")
    {
        return None;
    }

    let parts: Vec<&str> = candidate.rsplitn(3, ':').collect();
    let (clean_path, line_no, col_no) = match parts.as_slice() {
        [col_str, line_str, path_part]
            if col_str.parse::<usize>().is_ok() && line_str.parse::<usize>().is_ok() =>
        {
            let col = col_str.parse::<usize>().ok();
            let line = line_str.parse::<usize>().ok();
            (*path_part, line, col)
        }
        [line_str, path_part] if line_str.parse::<usize>().is_ok() => {
            let line = line_str.parse::<usize>().ok();
            (*path_part, line, None)
        }
        _ => (candidate, None, None),
    };

    // Verify path component still contains slash or starts with '~/'
    if !clean_path.contains('/') && !clean_path.starts_with("~/") {
        return None;
    }

    // Reject pure numeric fractions or dates (e.g. 1/2, 12/31/2024)
    let slash_segments = clean_path.split('/');
    let all_numeric = slash_segments
        .filter(|s| !s.is_empty())
        .all(|s| s.chars().all(|c| c.is_ascii_digit()));
    if all_numeric {
        return None;
    }

    // Path must contain allowed filesystem characters
    let valid_chars = clean_path.chars().all(|c| {
        c.is_alphanumeric()
            || matches!(
                c,
                '/' | '_' | '-' | '.' | '~' | '+' | '@' | '#' | '$' | '%' | '^'
            )
    });
    if !valid_chars {
        return None;
    }

    Some((clean_path.to_string(), line_no, col_no))
}

/// Extracts file paths with slashes, optionally with line and column numbers.
fn extract_paths(
    line: &str,
    screen_row: usize,
    pane_id: &str,
    occupied_ranges: &[(usize, usize)],
    tokens: &mut Vec<ExtractedToken>,
) {
    for (word_start, word) in WordsWithOffsets::new(line) {
        if is_overlapping(word_start, word_start + word.len(), occupied_ranges) {
            continue;
        }

        // Strip leading punctuation
        let leading_len = word
            .char_indices()
            .take_while(|(_, c)| matches!(*c, '(' | '[' | '{' | '<' | '"' | '\'' | '`'))
            .map(|(idx, c)| idx + c.len_utf8())
            .last()
            .unwrap_or(0);

        let trimmed_leading = &word[leading_len..];

        // Strip trailing punctuation
        let mut candidate = trimmed_leading;
        while let Some(last_ch) = candidate.chars().next_back() {
            if matches!(
                last_ch,
                ',' | '.' | ')' | ';' | ']' | '}' | '>' | ':' | '\'' | '"' | '`'
            ) {
                candidate = &candidate[..candidate.len() - last_ch.len_utf8()];
            } else {
                break;
            }
        }

        if candidate.is_empty() {
            continue;
        }

        let cand_start = word_start + leading_len;
        let cand_end = cand_start + candidate.len();

        if is_overlapping(cand_start, cand_end, occupied_ranges) {
            continue;
        }

        if let Some((clean_path, line_number, col_number)) = parse_path_candidate(candidate) {
            let col_start = line[..cand_start].chars().count();
            let col_end = col_start + candidate.chars().count();

            tokens.push(ExtractedToken {
                kind: EntityKind::Path,
                raw_text: candidate.to_string(),
                clean_text: clean_path,
                line_number,
                col_number,
                pane_id: pane_id.to_string(),
                screen_row,
                col_start,
                col_end,
            });
        }
    }
}

/// Extracts git commit hashes and hex identifiers (7 to 40 hex digits, not purely numeric).
fn extract_hashes(
    line: &str,
    screen_row: usize,
    pane_id: &str,
    occupied_ranges: &[(usize, usize)],
    tokens: &mut Vec<ExtractedToken>,
) {
    for (word_start, word) in WordsWithOffsets::new(line) {
        if is_overlapping(word_start, word_start + word.len(), occupied_ranges) {
            continue;
        }

        // Strip surrounding punctuation delimiters
        let leading_len = word
            .char_indices()
            .take_while(|(_, c)| matches!(*c, '(' | '[' | '{' | '<' | '"' | '\'' | '`' | ':' | '='))
            .map(|(idx, c)| idx + c.len_utf8())
            .last()
            .unwrap_or(0);

        let trimmed_leading = &word[leading_len..];

        let mut candidate = trimmed_leading;
        while let Some(last_ch) = candidate.chars().next_back() {
            if matches!(
                last_ch,
                ',' | '.' | ')' | ';' | ']' | '}' | '>' | ':' | '\'' | '"' | '`'
            ) {
                candidate = &candidate[..candidate.len() - last_ch.len_utf8()];
            } else {
                break;
            }
        }

        // Check hash length: 7 to 40 hex characters
        if candidate.len() < 7 || candidate.len() > 40 {
            continue;
        }

        // Must consist only of hex characters
        if !candidate.chars().all(|c| c.is_ascii_hexdigit()) {
            continue;
        }

        // Must contain at least one hex letter (a-f, A-F) to avoid numeric values/timestamps
        let has_hex_letter = candidate
            .chars()
            .any(|c| matches!(c, 'a'..='f' | 'A'..='F'));
        if !has_hex_letter {
            continue;
        }

        let cand_start = word_start + leading_len;
        let cand_end = cand_start + candidate.len();

        if is_overlapping(cand_start, cand_end, occupied_ranges) {
            continue;
        }

        // Verify boundary characters in line are not alphanumeric
        let before_is_alphanumeric = cand_start > 0
            && line[..cand_start]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric());

        let after_is_alphanumeric = cand_end < line.len()
            && line[cand_end..]
                .chars()
                .next()
                .is_some_and(|c| c.is_alphanumeric());

        if before_is_alphanumeric || after_is_alphanumeric {
            continue;
        }

        let col_start = line[..cand_start].chars().count();
        let col_end = col_start + candidate.chars().count();

        tokens.push(ExtractedToken {
            kind: EntityKind::Hash,
            raw_text: candidate.to_string(),
            clean_text: candidate.to_string(),
            line_number: None,
            col_number: None,
            pane_id: pane_id.to_string(),
            screen_row,
            col_start,
            col_end,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_various_paths() {
        let line = "file: /var/log/syslog:100:20 and relative ../foo/bar.rs:55 or ~/script.py";
        let tokens = scan_line(line, 0, "%0");

        let syslog = tokens
            .iter()
            .find(|t| t.clean_text == "/var/log/syslog")
            .unwrap();
        assert_eq!(syslog.kind, EntityKind::Path);
        assert_eq!(syslog.line_number, Some(100));
        assert_eq!(syslog.col_number, Some(20));

        let rel = tokens
            .iter()
            .find(|t| t.clean_text == "../foo/bar.rs")
            .unwrap();
        assert_eq!(rel.kind, EntityKind::Path);
        assert_eq!(rel.line_number, Some(55));
        assert_eq!(rel.col_number, None);

        let home = tokens
            .iter()
            .find(|t| t.clean_text == "~/script.py")
            .unwrap();
        assert_eq!(home.kind, EntityKind::Path);
        assert_eq!(home.line_number, None);
    }

    #[test]
    fn test_extract_git_url_and_hash() {
        let line = "clone git@github.com:org/repo.git at commit 5a4b3c2d1e";
        let tokens = scan_line(line, 0, "%0");

        assert!(tokens.iter().any(|t| t.kind == EntityKind::Url
            && t.clean_text == "git@github.com:org/repo.git"));
        assert!(
            tokens
                .iter()
                .any(|t| t.kind == EntityKind::Hash && t.clean_text == "5a4b3c2d1e")
        );
    }

    #[test]
    fn test_numeric_fractions_not_paths() {
        let line = "completed 5/10 tasks on 12/31/2024 with 1000000000";
        let tokens = scan_line(line, 0, "%0");
        assert!(tokens.iter().all(|t| t.kind != EntityKind::Path));
        assert!(tokens.iter().all(|t| t.kind != EntityKind::Hash));
    }
}

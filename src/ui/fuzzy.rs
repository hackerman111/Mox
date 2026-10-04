//! Shared subsequence scoring, independent of feature models.
pub fn fuzzy_match(target: &str, query: &str) -> Option<i64> {
    if query.is_empty() {
        return Some(0);
    }
    let query_len = query.chars().count();
    let mut query = query.chars().peekable();
    let mut score = 0;
    let mut previous = None;
    let mut last_match = None;
    let mut length: usize = 0;
    for (index, ch) in target.chars().enumerate() {
        length += 1;
        if query.peek().is_some_and(|q| q.eq_ignore_ascii_case(&ch)) {
            score += 10;
            if index == 0 {
                score += 20;
            }
            if previous.is_some_and(|p| matches!(p, ' ' | '/' | '-' | '_' | '.')) {
                score += 15;
            }
            if last_match == Some(index.saturating_sub(1)) {
                score += 10;
            }
            last_match = Some(index);
            query.next();
        }
        previous = Some(ch);
    }
    if query.peek().is_some() {
        None
    } else {
        Some(score - length.saturating_sub(query_len) as i64)
    }
}

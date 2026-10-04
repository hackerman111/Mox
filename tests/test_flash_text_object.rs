use mox::flash::text_object::{TextObject, resolve_text_object, resolve_text_object_span};

#[test]
fn test_resolve_word_and_token_objects() {
    let line = "let candidate = \"https://antigravity.dev\";";
    // Target is inside the URL (col 20 is within the url)
    let col = 20;
    let resolved = resolve_text_object(line, col, TextObject::AutoToken).unwrap();
    assert_eq!(resolved, "https://antigravity.dev");

    // Inner word at col 4 ("candidate")
    let word = resolve_text_object(line, 4, TextObject::InnerWord).unwrap();
    assert_eq!(word, "candidate");

    // A word at col 4
    let aword = resolve_text_object(line, 4, TextObject::AWord).unwrap();
    assert_eq!(aword, "candidate ");
}

#[test]
fn test_resolve_quoted_objects() {
    let line = "error: \"file not found\" in path";
    let col = 10; // inside "file not found"

    let inner = resolve_text_object(line, col, TextObject::InnerQuoted).unwrap();
    assert_eq!(inner, "file not found");

    let a_quote = resolve_text_object(line, col, TextObject::AQuoted).unwrap();
    assert_eq!(a_quote, "\"file not found\"");
}

#[test]
fn test_resolve_line_and_to_end() {
    let line = "  first second third  ";
    let line_obj = resolve_text_object(line, 5, TextObject::Line).unwrap();
    assert_eq!(line_obj, "first second third");

    let to_end = resolve_text_object(line, 8, TextObject::ToEndOfLine).unwrap();
    assert_eq!(to_end, "second third  ");
}

#[test]
fn test_resolve_text_object_span() {
    let line = "foo bar baz";
    let (start, end, text) = resolve_text_object_span(line, 4, TextObject::InnerWord).unwrap();
    assert_eq!(start, 4);
    assert_eq!(end, 7);
    assert_eq!(text, "bar");
}

#[test]
fn test_resolve_bracket_object() {
    let line = "fn compute(x + y * 2) -> i32";
    let col = 14; // inside (x + y * 2)
    let (start, end, text) = resolve_text_object_span(line, col, TextObject::Bracket).unwrap();
    assert_eq!(start, 10);
    assert_eq!(end, 21);
    assert_eq!(text, "(x + y * 2)");
}

#[test]
fn test_resolve_aword_at_end_of_line() {
    let line = "hello world";
    // "world" is at end of line, so AWord should include leading whitespace
    let aword = resolve_text_object(line, 8, TextObject::AWord).unwrap();
    assert_eq!(aword, " world");
}

#[test]
fn test_resolve_auto_token_fallback() {
    let line = "simple_identifier here";
    let resolved = resolve_text_object(line, 2, TextObject::AutoToken).unwrap();
    assert_eq!(resolved, "simple_identifier");
}

#[test]
fn test_resolve_single_quotes_and_backticks() {
    let line = "run `cargo test` and 'npm test'";
    // Backtick
    let inner_bt = resolve_text_object(line, 8, TextObject::InnerQuoted).unwrap();
    assert_eq!(inner_bt, "cargo test");
    let a_bt = resolve_text_object(line, 8, TextObject::AQuoted).unwrap();
    assert_eq!(a_bt, "`cargo test`");

    // Single quote
    let inner_sq = resolve_text_object(line, 25, TextObject::InnerQuoted).unwrap();
    assert_eq!(inner_sq, "npm test");
    let a_sq = resolve_text_object(line, 25, TextObject::AQuoted).unwrap();
    assert_eq!(a_sq, "'npm test'");
}

#[test]
fn test_resolve_nested_brackets() {
    let line = "vec![1, (2 + 3), 4]";
    // Inside inner paren (col 11 is inside `(2 + 3)`)
    let inner_bracket = resolve_text_object(line, 11, TextObject::Bracket).unwrap();
    assert_eq!(inner_bracket, "(2 + 3)");

    // Outside inner paren but inside `[...]` (col 5 is at `1`)
    let outer_bracket = resolve_text_object(line, 5, TextObject::Bracket).unwrap();
    assert_eq!(outer_bracket, "[1, (2 + 3), 4]");
}

#[test]
fn test_resolve_utf8_strings() {
    let line = "🦀 const переменная_var = \"тест\";";
    // Target `переменная_var` at char index 10 (inside Cyrillic word)
    let word = resolve_text_object(line, 10, TextObject::InnerWord).unwrap();
    assert_eq!(word, "переменная_var");

    // Target inside Cyrillic quote `"тест"` at char index 26
    let quoted = resolve_text_object(line, 26, TextObject::InnerQuoted).unwrap();
    assert_eq!(quoted, "тест");
}

#[test]
fn test_resolve_out_of_bounds_and_none() {
    let line = "foo bar";
    assert!(resolve_text_object(line, 100, TextObject::InnerWord).is_none());
    assert!(resolve_text_object(line, 3, TextObject::InnerWord).is_none()); // on whitespace
    assert!(resolve_text_object(line, 0, TextObject::Bracket).is_none()); // no brackets
    assert!(resolve_text_object(line, 0, TextObject::InnerQuoted).is_none()); // no quotes
}

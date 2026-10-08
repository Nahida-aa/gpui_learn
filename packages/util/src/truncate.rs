/// Returns whether the byte value is a valid UTF-8 char boundary.
#[inline]
pub const fn is_utf8_char_boundary(u8: u8) -> bool {
    // This is bit magic equivalent to: b < 128 || b >= 192
    (u8 as i8) >= -0x40
}

pub fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}

/// Removes characters from the end of the string if its length is greater than `max_chars` and
/// appends "..." to the string. Returns string unchanged if its length is smaller than max_chars.
pub fn truncate_and_trailoff(s: &str, max_chars: usize) -> String {
    debug_assert!(max_chars >= 5);

    // If the string's byte length is <= max_chars, walking the string can be skipped since the
    // number of chars is <= the number of bytes.
    if s.len() <= max_chars {
        return s.to_string();
    }
    let truncation_ix = s.char_indices().map(|(i, _)| i).nth(max_chars);
    match truncation_ix {
        Some(index) => s[..index].to_string() + "…",
        _ => s.to_string(),
    }
}

/// Removes characters from the front of the string if its length is greater than `max_chars` and
/// prepends the string with "...". Returns string unchanged if its length is smaller than max_chars.
pub fn truncate_and_remove_front(s: &str, max_chars: usize) -> String {
    debug_assert!(max_chars >= 5);

    // If the string's byte length is <= max_chars, walking the string can be skipped since the
    // number of chars is <= the number of bytes.
    if s.len() <= max_chars {
        return s.to_string();
    }
    let suffix_char_length = max_chars.saturating_sub(1);
    let truncation_ix = s
        .char_indices()
        .map(|(i, _)| i)
        .nth_back(suffix_char_length);
    match truncation_ix {
        Some(index) if index > 0 => "…".to_string() + &s[index..],
        _ => s.to_string(),
    }
}

/// Takes only `max_lines` from the string and, if there were more than `max_lines-1`, appends a
/// a newline and "..." to the string, so that `max_lines` are returned.
/// Returns string unchanged if its length is smaller than max_lines.
pub fn truncate_lines_and_trailoff(s: &str, max_lines: usize) -> String {
    let mut lines = s.lines().take(max_lines).collect::<Vec<_>>();
    if lines.len() > max_lines - 1 {
        lines.pop();
        lines.join("\n") + "\n…"
    } else {
        lines.join("\n")
    }
}

/// Truncates the string at a character boundary, such that the result is less than `max_bytes` in
/// length.
pub fn truncate_to_byte_limit(s: &str, max_bytes: usize) -> &str {
    if s.len() < max_bytes {
        return s;
    }

    for i in (0..max_bytes).rev() {
        if s.is_char_boundary(i) {
            return &s[..i];
        }
    }

    ""
}

/// Takes a prefix of complete lines which fit within the byte limit. If the first line is longer
/// than the limit, truncates at a character boundary.
pub fn truncate_lines_to_byte_limit(s: &str, max_bytes: usize) -> &str {
    if s.len() < max_bytes {
        return s;
    }

    for i in (0..max_bytes).rev() {
        if s.is_char_boundary(i) && s.as_bytes()[i] == b'\n' {
            // Since the i-th character is \n, valid to slice at i + 1.
            return &s[..i + 1];
        }
    }

    truncate_to_byte_limit(s, max_bytes)
}

#[test]
fn test_truncate_lines_to_byte_limit() {
    let text = "Line 1\nLine 2\nLine 3\nLine 4";

    // Limit that includes all lines
    assert_eq!(truncate_lines_to_byte_limit(text, 100), text);

    // Exactly the first line
    assert_eq!(truncate_lines_to_byte_limit(text, 7), "Line 1\n");

    // Limit between lines
    assert_eq!(truncate_lines_to_byte_limit(text, 13), "Line 1\n");
    assert_eq!(truncate_lines_to_byte_limit(text, 20), "Line 1\nLine 2\n");

    // Limit before first newline
    assert_eq!(truncate_lines_to_byte_limit(text, 6), "Line ");

    // Test with non-ASCII characters
    let text_utf8 = "Line 1\nLíne 2\nLine 3";
    assert_eq!(
        truncate_lines_to_byte_limit(text_utf8, 15),
        "Line 1\nLíne 2\n"
    );
}

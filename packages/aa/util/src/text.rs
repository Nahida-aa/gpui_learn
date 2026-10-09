use std::ops::Range;
use std::sync::LazyLock;

use regex::Regex;

fn emoji_regex() -> &'static Regex {
    static EMOJI_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new("(\\p{Emoji}|\u{200D})").unwrap());
    &EMOJI_REGEX
}

/// Returns true if the given string consists of emojis only.
/// E.g. "👨‍👩‍👧‍👧👋" will return true, but "👋!" will return false.
pub fn word_consists_of_emojis(s: &str) -> bool {
    let mut prev_end = 0;
    for capture in emoji_regex().find_iter(s) {
        if capture.start() != prev_end {
            return false;
        }
        prev_end = capture.end();
    }
    prev_end == s.len()
}

/// Similar to `str::split`, but also provides byte-offset ranges of the results. Unlike
/// `str::split`, this is not generic on pattern types and does not return an `Iterator`.
pub fn split_str_with_ranges<'s>(
    s: &'s str,
    pat: &dyn Fn(char) -> bool,
) -> Vec<(Range<usize>, &'s str)> {
    let mut result = Vec::new();
    let mut start = 0;

    for (i, ch) in s.char_indices() {
        if pat(ch) {
            if i > start {
                result.push((start..i, &s[start..i]));
            }
            start = i + ch.len_utf8();
        }
    }

    if s.len() > start {
        result.push((start..s.len(), &s[start..s.len()]));
    }

    result
}

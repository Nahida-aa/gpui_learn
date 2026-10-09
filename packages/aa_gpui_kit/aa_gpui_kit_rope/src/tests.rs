use super::*;
use Bias::{Left, Right};
use rand::prelude::*;
use std::{cmp::Ordering, env, io::Read};
use sum_tree::Bias;
use util::RandomCharIter;

#[ctor::ctor(unsafe)]
fn init_logger() { zlog::init_test(); }

#[test]
fn test_all_4_byte_chars() {
    let mut rope = Rope::new();
    let text = "🏀".repeat(256);
    rope.push(&text);
    assert_eq!(rope.text(), text);
}

#[test]
fn test_clip() {
    let rope = Rope::from("🧘");

    assert_eq!(rope.clip_offset(1, Bias::Left), 0);
    assert_eq!(rope.clip_offset(1, Bias::Right), 4);
    assert_eq!(rope.clip_offset(5, Bias::Right), 4);

    assert_eq!(
        rope.clip_point(Point::new(0, 1), Bias::Left),
        Point::new(0, 0)
    );
    assert_eq!(
        rope.clip_point(Point::new(0, 1), Bias::Right),
        Point::new(0, 4)
    );
    assert_eq!(
        rope.clip_point(Point::new(0, 5), Bias::Right),
        Point::new(0, 4)
    );

    assert_eq!(
        rope.clip_point_utf16(Unclipped(PointUtf16::new(0, 1)), Bias::Left),
        PointUtf16::new(0, 0)
    );
    assert_eq!(
        rope.clip_point_utf16(Unclipped(PointUtf16::new(0, 1)), Bias::Right),
        PointUtf16::new(0, 2)
    );
    assert_eq!(
        rope.clip_point_utf16(Unclipped(PointUtf16::new(0, 3)), Bias::Right),
        PointUtf16::new(0, 2)
    );

    assert_eq!(
        rope.clip_offset_utf16(OffsetUtf16(1), Bias::Left),
        OffsetUtf16(0)
    );
    assert_eq!(
        rope.clip_offset_utf16(OffsetUtf16(1), Bias::Right),
        OffsetUtf16(2)
    );
    assert_eq!(
        rope.clip_offset_utf16(OffsetUtf16(3), Bias::Right),
        OffsetUtf16(2)
    );
}

#[test]
fn test_prev_next_line() {
    let rope = Rope::from("abc\ndef\nghi\njkl");

    let mut chunks = rope.chunks();
    assert_eq!(chunks.peek().unwrap().chars().next().unwrap(), 'a');

    assert!(chunks.next_line());
    assert_eq!(chunks.peek().unwrap().chars().next().unwrap(), 'd');

    assert!(chunks.next_line());
    assert_eq!(chunks.peek().unwrap().chars().next().unwrap(), 'g');

    assert!(chunks.next_line());
    assert_eq!(chunks.peek().unwrap().chars().next().unwrap(), 'j');

    assert!(!chunks.next_line());
    assert_eq!(chunks.peek(), None);

    assert!(chunks.prev_line());
    assert_eq!(chunks.peek().unwrap().chars().next().unwrap(), 'j');

    assert!(chunks.prev_line());
    assert_eq!(chunks.peek().unwrap().chars().next().unwrap(), 'g');

    assert!(chunks.prev_line());
    assert_eq!(chunks.peek().unwrap().chars().next().unwrap(), 'd');

    assert!(chunks.prev_line());
    assert_eq!(chunks.peek().unwrap().chars().next().unwrap(), 'a');

    assert!(!chunks.prev_line());
    assert_eq!(chunks.peek().unwrap().chars().next().unwrap(), 'a');

    // Only return true when the cursor has moved to the start of a line
    let mut chunks = rope.chunks_in_range(5..7);
    chunks.seek(6);
    assert!(!chunks.prev_line());
    assert_eq!(chunks.peek().unwrap().chars().next().unwrap(), 'e');

    assert!(!chunks.next_line());
    assert_eq!(chunks.peek(), None);
}

#[test]
fn test_lines() {
    let rope = Rope::from("abc\ndefg\nhi");
    let mut lines = rope.chunks().lines();
    assert_eq!(lines.next(), Some("abc"));
    assert_eq!(lines.next(), Some("defg"));
    assert_eq!(lines.next(), Some("hi"));
    assert_eq!(lines.next(), None);

    let rope = Rope::from("abc\ndefg\nhi\n");
    let mut lines = rope.chunks().lines();
    assert_eq!(lines.next(), Some("abc"));
    assert_eq!(lines.next(), Some("defg"));
    assert_eq!(lines.next(), Some("hi"));
    assert_eq!(lines.next(), Some(""));
    assert_eq!(lines.next(), None);

    let rope = Rope::from("abc\ndefg\nhi");
    let mut lines = rope.reversed_chunks_in_range(0..rope.len()).lines();
    assert_eq!(lines.next(), Some("hi"));
    assert_eq!(lines.next(), Some("defg"));
    assert_eq!(lines.next(), Some("abc"));
    assert_eq!(lines.next(), None);

    let rope = Rope::from("abc\ndefg\nhi\n");
    let mut lines = rope.reversed_chunks_in_range(0..rope.len()).lines();
    assert_eq!(lines.next(), Some(""));
    assert_eq!(lines.next(), Some("hi"));
    assert_eq!(lines.next(), Some("defg"));
    assert_eq!(lines.next(), Some("abc"));
    assert_eq!(lines.next(), None);

    let rope = Rope::from("abc\nlonger line test\nhi");
    let mut lines = rope.chunks().lines();
    assert_eq!(lines.next(), Some("abc"));
    assert_eq!(lines.next(), Some("longer line test"));
    assert_eq!(lines.next(), Some("hi"));
    assert_eq!(lines.next(), None);

    let rope = Rope::from("abc\nlonger line test\nhi");
    let mut lines = rope.reversed_chunks_in_range(0..rope.len()).lines();
    assert_eq!(lines.next(), Some("hi"));
    assert_eq!(lines.next(), Some("longer line test"));
    assert_eq!(lines.next(), Some("abc"));
    assert_eq!(lines.next(), None);
}

#[gpui::test(iterations = 100)]
fn test_random_rope(mut rng: StdRng) {
    let operations = env::var("OPERATIONS")
        .map(|i| i.parse().expect("invalid `OPERATIONS` variable"))
        .unwrap_or(10);

    let mut expected = String::new();
    let mut actual = Rope::new();
    for _ in 0..operations {
        let end_ix = clip_offset(&expected, rng.random_range(0..=expected.len()), Right);
        let start_ix = clip_offset(&expected, rng.random_range(0..=end_ix), Left);
        let len = rng.random_range(0..=64);
        let new_text: String = RandomCharIter::new(&mut rng).take(len).collect();

        let mut new_actual = Rope::new();
        let mut cursor = actual.cursor(0);
        new_actual.append(cursor.slice(start_ix));
        new_actual.push(&new_text);
        cursor.seek_forward(end_ix);
        new_actual.append(cursor.suffix());
        actual = new_actual;

        expected.replace_range(start_ix..end_ix, &new_text);

        assert_eq!(actual.text(), expected);
        log::info!("text: {:?}", expected);

        for _ in 0..5 {
            let end_ix = clip_offset(&expected, rng.random_range(0..=expected.len()), Right);
            let start_ix = clip_offset(&expected, rng.random_range(0..=end_ix), Left);

            let actual_text = actual.chunks_in_range(start_ix..end_ix).collect::<String>();
            assert_eq!(actual_text, &expected[start_ix..end_ix]);

            let mut actual_text = String::new();
            actual
                .bytes_in_range(start_ix..end_ix)
                .read_to_string(&mut actual_text)
                .unwrap();
            assert_eq!(actual_text, &expected[start_ix..end_ix]);

            assert_eq!(
                actual
                    .reversed_chunks_in_range(start_ix..end_ix)
                    .collect::<Vec<&str>>()
                    .into_iter()
                    .rev()
                    .collect::<String>(),
                &expected[start_ix..end_ix]
            );

            let mut expected_line_starts: Vec<_> = expected[start_ix..end_ix]
                .match_indices('\n')
                .map(|(index, _)| start_ix + index + 1)
                .collect();

            let mut chunks = actual.chunks_in_range(start_ix..end_ix);

            let mut actual_line_starts = Vec::new();
            while chunks.next_line() {
                actual_line_starts.push(chunks.offset());
            }
            assert_eq!(
                actual_line_starts,
                expected_line_starts,
                "actual line starts != expected line starts when using next_line() for {:?} ({:?})",
                &expected[start_ix..end_ix],
                start_ix..end_ix
            );

            if start_ix < end_ix && (start_ix == 0 || expected.as_bytes()[start_ix - 1] == b'\n') {
                expected_line_starts.insert(0, start_ix);
            }
            // Remove the last index if it starts at the end of the range.
            if expected_line_starts.last() == Some(&end_ix) {
                expected_line_starts.pop();
            }

            let mut actual_line_starts = Vec::new();
            while chunks.prev_line() {
                actual_line_starts.push(chunks.offset());
            }
            actual_line_starts.reverse();
            assert_eq!(
                actual_line_starts,
                expected_line_starts,
                "actual line starts != expected line starts when using prev_line() for {:?} ({:?})",
                &expected[start_ix..end_ix],
                start_ix..end_ix
            );

            // Check that next_line/prev_line work correctly from random positions
            let mut offset = rng.random_range(start_ix..=end_ix);
            while !expected.is_char_boundary(offset) {
                offset -= 1;
            }
            chunks.seek(offset);

            for _ in 0..5 {
                if rng.random() {
                    let expected_next_line_start = expected[offset..end_ix]
                        .find('\n')
                        .map(|newline_ix| offset + newline_ix + 1);

                    let moved = chunks.next_line();
                    assert_eq!(
                        moved,
                        expected_next_line_start.is_some(),
                        "unexpected result from next_line after seeking to {} in range {:?} ({:?})",
                        offset,
                        start_ix..end_ix,
                        &expected[start_ix..end_ix]
                    );
                    if let Some(expected_next_line_start) = expected_next_line_start {
                        assert_eq!(
                            chunks.offset(),
                            expected_next_line_start,
                            "invalid position after seeking to {} in range {:?} ({:?})",
                            offset,
                            start_ix..end_ix,
                            &expected[start_ix..end_ix]
                        );
                    } else {
                        assert_eq!(
                            chunks.offset(),
                            end_ix,
                            "invalid position after seeking to {} in range {:?} ({:?})",
                            offset,
                            start_ix..end_ix,
                            &expected[start_ix..end_ix]
                        );
                    }
                } else {
                    let search_end = if offset > 0 && expected.as_bytes()[offset - 1] == b'\n' {
                        offset - 1
                    } else {
                        offset
                    };

                    let expected_prev_line_start = expected[..search_end]
                        .rfind('\n')
                        .and_then(|newline_ix| {
                            let line_start_ix = newline_ix + 1;
                            if line_start_ix >= start_ix {
                                Some(line_start_ix)
                            } else {
                                None
                            }
                        })
                        .or({
                            if offset > 0 && start_ix == 0 {
                                Some(0)
                            } else {
                                None
                            }
                        });

                    let moved = chunks.prev_line();
                    assert_eq!(
                        moved,
                        expected_prev_line_start.is_some(),
                        "unexpected result from prev_line after seeking to {} in range {:?} ({:?})",
                        offset,
                        start_ix..end_ix,
                        &expected[start_ix..end_ix]
                    );
                    if let Some(expected_prev_line_start) = expected_prev_line_start {
                        assert_eq!(
                            chunks.offset(),
                            expected_prev_line_start,
                            "invalid position after seeking to {} in range {:?} ({:?})",
                            offset,
                            start_ix..end_ix,
                            &expected[start_ix..end_ix]
                        );
                    } else {
                        assert_eq!(
                            chunks.offset(),
                            start_ix,
                            "invalid position after seeking to {} in range {:?} ({:?})",
                            offset,
                            start_ix..end_ix,
                            &expected[start_ix..end_ix]
                        );
                    }
                }

                assert!((start_ix..=end_ix).contains(&chunks.offset()));
                if rng.random() {
                    offset = rng.random_range(start_ix..=end_ix);
                    while !expected.is_char_boundary(offset) {
                        offset -= 1;
                    }
                    chunks.seek(offset);
                } else {
                    chunks.next();
                    offset = chunks.offset();
                    assert!((start_ix..=end_ix).contains(&chunks.offset()));
                }
            }
        }

        let mut offset_utf16 = OffsetUtf16(0);
        let mut point = Point::new(0, 0);
        let mut point_utf16 = PointUtf16::new(0, 0);
        for (ix, ch) in expected.char_indices().chain(Some((expected.len(), '\0'))) {
            assert_eq!(actual.offset_to_point(ix), point, "offset_to_point({})", ix);
            assert_eq!(
                actual.offset_to_point_utf16(ix),
                point_utf16,
                "offset_to_point_utf16({})",
                ix
            );
            assert_eq!(
                actual.point_to_offset(point),
                ix,
                "point_to_offset({:?})",
                point
            );
            assert_eq!(
                actual.point_utf16_to_offset(point_utf16),
                ix,
                "point_utf16_to_offset({:?})",
                point_utf16
            );
            assert_eq!(
                actual.offset_to_offset_utf16(ix),
                offset_utf16,
                "offset_to_offset_utf16({:?})",
                ix
            );
            assert_eq!(
                actual.offset_utf16_to_offset(offset_utf16),
                ix,
                "offset_utf16_to_offset({:?})",
                offset_utf16
            );
            if ch == '\n' {
                point += Point::new(1, 0);
                point_utf16 += PointUtf16::new(1, 0);
            } else {
                point.column += ch.len_utf8() as u32;
                point_utf16.column += ch.len_utf16() as u32;
            }
            offset_utf16.0 += ch.len_utf16();
        }

        let mut offset_utf16 = OffsetUtf16(0);
        let mut point_utf16 = Unclipped(PointUtf16::zero());
        for unit in expected.encode_utf16() {
            let left_offset = actual.clip_offset_utf16(offset_utf16, Bias::Left);
            let right_offset = actual.clip_offset_utf16(offset_utf16, Bias::Right);
            assert!(right_offset >= left_offset);
            // Ensure translating UTF-16 offsets to UTF-8 offsets doesn't panic.
            actual.offset_utf16_to_offset(left_offset);
            actual.offset_utf16_to_offset(right_offset);

            let left_point = actual.clip_point_utf16(point_utf16, Bias::Left);
            let right_point = actual.clip_point_utf16(point_utf16, Bias::Right);
            assert!(right_point >= left_point);
            // Ensure translating valid UTF-16 points to offsets doesn't panic.
            actual.point_utf16_to_offset(left_point);
            actual.point_utf16_to_offset(right_point);

            offset_utf16.0 += 1;
            if unit == b'\n' as u16 {
                point_utf16.0 += PointUtf16::new(1, 0);
            } else {
                point_utf16.0 += PointUtf16::new(0, 1);
            }
        }

        for _ in 0..5 {
            let end_ix = clip_offset(&expected, rng.random_range(0..=expected.len()), Right);
            let start_ix = clip_offset(&expected, rng.random_range(0..=end_ix), Left);
            assert_eq!(
                actual.cursor(start_ix).summary::<TextSummary>(end_ix),
                TextSummary::from(&expected[start_ix..end_ix])
            );
        }

        let mut expected_longest_rows = Vec::new();
        let mut longest_line_len = -1_isize;
        for (row, line) in expected.split('\n').enumerate() {
            let row = row as u32;
            assert_eq!(
                actual.line_len(row),
                line.len() as u32,
                "invalid line len for row {}",
                row
            );

            let line_char_count = line.chars().count() as isize;
            match line_char_count.cmp(&longest_line_len) {
                Ordering::Less => {}
                Ordering::Equal => expected_longest_rows.push(row),
                Ordering::Greater => {
                    longest_line_len = line_char_count;
                    expected_longest_rows.clear();
                    expected_longest_rows.push(row);
                }
            }
        }

        let longest_row = actual.summary().longest_row;
        assert!(
            expected_longest_rows.contains(&longest_row),
            "incorrect longest row {}. expected {:?} with length {}",
            longest_row,
            expected_longest_rows,
            longest_line_len,
        );
    }
}

#[test]
fn test_chunks_equals_str() {
    let text = "This is a multi-chunk\n& multi-line test string!";
    let rope = Rope::from(text);
    for start in 0..text.len() {
        for end in start..text.len() {
            let range = start..end;
            let correct_substring = &text[start..end];

            // Test that correct range returns true
            assert!(
                rope.chunks_in_range(range.clone())
                    .equals_str(correct_substring)
            );
            assert!(
                rope.reversed_chunks_in_range(range.clone())
                    .equals_str(correct_substring)
            );

            // Test that all other ranges return false (unless they happen to match)
            for other_start in 0..text.len() {
                for other_end in other_start..text.len() {
                    if other_start == start && other_end == end {
                        continue;
                    }
                    let other_substring = &text[other_start..other_end];

                    // Only assert false if the substrings are actually different
                    if other_substring == correct_substring {
                        continue;
                    }
                    assert!(
                        !rope
                            .chunks_in_range(range.clone())
                            .equals_str(other_substring)
                    );
                    assert!(
                        !rope
                            .reversed_chunks_in_range(range.clone())
                            .equals_str(other_substring)
                    );
                }
            }
        }
    }

    let rope = Rope::from("");
    assert!(rope.chunks_in_range(0..0).equals_str(""));
    assert!(rope.reversed_chunks_in_range(0..0).equals_str(""));
    assert!(!rope.chunks_in_range(0..0).equals_str("foo"));
    assert!(!rope.reversed_chunks_in_range(0..0).equals_str("foo"));
}

#[test]
fn test_starts_with() {
    let text = "Hello, world! 🌍🌎🌏";
    let rope = Rope::from(text);

    assert!(rope.starts_with(""));
    assert!(rope.starts_with("H"));
    assert!(rope.starts_with("Hello"));
    assert!(rope.starts_with("Hello, world! 🌍🌎🌏"));
    assert!(!rope.starts_with("ello"));
    assert!(!rope.starts_with("Hello, world! 🌍🌎🌏!"));

    let empty_rope = Rope::from("");
    assert!(empty_rope.starts_with(""));
    assert!(!empty_rope.starts_with("a"));
}

#[test]
fn test_ends_with() {
    let text = "Hello, world! 🌍🌎🌏";
    let rope = Rope::from(text);

    assert!(rope.ends_with(""));
    assert!(rope.ends_with("🌏"));
    assert!(rope.ends_with("🌍🌎🌏"));
    assert!(rope.ends_with("Hello, world! 🌍🌎🌏"));
    assert!(!rope.ends_with("🌎"));
    assert!(!rope.ends_with("!Hello, world! 🌍🌎🌏"));

    let empty_rope = Rope::from("");
    assert!(empty_rope.ends_with(""));
    assert!(!empty_rope.ends_with("a"));
}

#[test]
fn test_starts_with_ends_with_random() {
    let mut rng = StdRng::seed_from_u64(0);
    for _ in 0..100 {
        let len = rng.random_range(0..100);
        let text: String = RandomCharIter::new(&mut rng).take(len).collect();
        let rope = Rope::from(text.as_str());

        for _ in 0..10 {
            let start = rng.random_range(0..=text.len());
            let start = text.ceil_char_boundary(start);
            let end = rng.random_range(start..=text.len());
            let end = text.ceil_char_boundary(end);
            let prefix = &text[..end];
            let suffix = &text[start..];

            assert_eq!(
                rope.starts_with(prefix),
                text.starts_with(prefix),
                "starts_with mismatch for {:?} in {:?}",
                prefix,
                text
            );
            assert_eq!(
                rope.ends_with(suffix),
                text.ends_with(suffix),
                "ends_with mismatch for {:?} in {:?}",
                suffix,
                text
            );
        }
    }
}

#[test]
fn test_is_char_boundary() {
    let fixture = "地";
    let rope = Rope::from("地");
    for b in 0..=fixture.len() {
        assert_eq!(rope.is_char_boundary(b), fixture.is_char_boundary(b));
    }
    let fixture = "";
    let rope = Rope::from("");
    for b in 0..=fixture.len() {
        assert_eq!(rope.is_char_boundary(b), fixture.is_char_boundary(b));
    }
    let fixture = "🔴🟠🟡🟢🔵🟣⚫️⚪️🟤\n🏳️‍⚧️🏁🏳️‍🌈🏴‍☠️⛳️📬📭🏴🏳️🚩";
    let rope = Rope::from("🔴🟠🟡🟢🔵🟣⚫️⚪️🟤\n🏳️‍⚧️🏁🏳️‍🌈🏴‍☠️⛳️📬📭🏴🏳️🚩");
    for b in 0..=fixture.len() {
        assert_eq!(rope.is_char_boundary(b), fixture.is_char_boundary(b));
    }
}

#[test]
fn test_floor_char_boundary() {
    let fixture = "地";
    let rope = Rope::from("地");
    for b in 0..=fixture.len() {
        assert_eq!(rope.floor_char_boundary(b), fixture.floor_char_boundary(b));
    }

    let fixture = "";
    let rope = Rope::from("");
    for b in 0..=fixture.len() {
        assert_eq!(rope.floor_char_boundary(b), fixture.floor_char_boundary(b));
    }

    let fixture = "🔴🟠🟡🟢🔵🟣⚫️⚪️🟤\n🏳️‍⚧️🏁🏳️‍🌈🏴‍☠️⛳️📬📭🏴🏳️🚩";
    let rope = Rope::from("🔴🟠🟡🟢🔵🟣⚫️⚪️🟤\n🏳️‍⚧️🏁🏳️‍🌈🏴‍☠️⛳️📬📭🏴🏳️🚩");
    for b in 0..=fixture.len() {
        assert_eq!(rope.floor_char_boundary(b), fixture.floor_char_boundary(b));
    }
}

#[test]
fn test_ceil_char_boundary() {
    let fixture = "地";
    let rope = Rope::from("地");
    for b in 0..=fixture.len() {
        assert_eq!(rope.ceil_char_boundary(b), fixture.ceil_char_boundary(b));
    }

    let fixture = "";
    let rope = Rope::from("");
    for b in 0..=fixture.len() {
        assert_eq!(rope.ceil_char_boundary(b), fixture.ceil_char_boundary(b));
    }

    let fixture = "🔴🟠🟡🟢🔵🟣⚫️⚪️🟤\n🏳️‍⚧️🏁🏳️‍🌈🏴‍☠️⛳️📬📭🏴🏳️🚩";
    let rope = Rope::from("🔴🟠🟡🟢🔵🟣⚫️⚪️🟤\n🏳️‍⚧️🏁🏳️‍🌈🏴‍☠️⛳️📬📭🏴🏳️🚩");
    for b in 0..=fixture.len() {
        assert_eq!(rope.ceil_char_boundary(b), fixture.ceil_char_boundary(b));
    }
}

#[test]
fn test_push_front_empty_text_on_empty_rope() {
    let mut rope = Rope::new();
    rope.push_front("");
    assert_eq!(rope.text(), "");
    assert_eq!(rope.len(), 0);
}

#[test]
fn test_push_front_empty_text_on_nonempty_rope() {
    let mut rope = Rope::from("hello");
    rope.push_front("");
    assert_eq!(rope.text(), "hello");
}

#[test]
fn test_push_front_on_empty_rope() {
    let mut rope = Rope::new();
    rope.push_front("hello");
    assert_eq!(rope.text(), "hello");
    assert_eq!(rope.len(), 5);
    assert_eq!(rope.max_point(), Point::new(0, 5));
}

#[test]
fn test_push_front_single_space() {
    let mut rope = Rope::from("hint");
    rope.push_front(" ");
    assert_eq!(rope.text(), " hint");
    assert_eq!(rope.len(), 5);
}

#[gpui::test(iterations = 50)]
fn test_push_front_random(mut rng: StdRng) {
    let initial_len = rng.random_range(0..=64);
    let initial_text: String = RandomCharIter::new(&mut rng).take(initial_len).collect();
    let mut rope = Rope::from(initial_text.as_str());

    let mut expected = initial_text;

    for _ in 0..rng.random_range(1..=10) {
        let prefix_len = rng.random_range(0..=32);
        let prefix: String = RandomCharIter::new(&mut rng).take(prefix_len).collect();

        rope.push_front(&prefix);
        expected.insert_str(0, &prefix);

        assert_eq!(
            rope.text(),
            expected,
            "text mismatch after push_front({:?})",
            prefix
        );
        assert_eq!(rope.len(), expected.len());

        let actual_summary = rope.summary();
        let expected_summary = TextSummary::from(expected.as_str());
        assert_eq!(
            actual_summary.len, expected_summary.len,
            "len mismatch for {:?}",
            expected
        );
        assert_eq!(
            actual_summary.lines, expected_summary.lines,
            "lines mismatch for {:?}",
            expected
        );
        assert_eq!(
            actual_summary.chars, expected_summary.chars,
            "chars mismatch for {:?}",
            expected
        );
        assert_eq!(
            actual_summary.longest_row, expected_summary.longest_row,
            "longest_row mismatch for {:?}",
            expected
        );

        // Verify offset-to-point and point-to-offset round-trip at boundaries.
        for (ix, _) in expected.char_indices().chain(Some((expected.len(), '\0'))) {
            assert_eq!(
                rope.point_to_offset(rope.offset_to_point(ix)),
                ix,
                "offset round-trip failed at {} for {:?}",
                ix,
                expected
            );
        }
    }
}

#[gpui::test(iterations = 50)]
fn test_push_front_large_prefix(mut rng: StdRng) {
    let initial_len = rng.random_range(0..=32);
    let initial_text: String = RandomCharIter::new(&mut rng).take(initial_len).collect();
    let mut rope = Rope::from(initial_text.as_str());

    let prefix_len = rng.random_range(64..=256);
    let prefix: String = RandomCharIter::new(&mut rng).take(prefix_len).collect();

    rope.push_front(&prefix);
    let expected = format!("{}{}", prefix, initial_text);

    assert_eq!(rope.text(), expected);
    assert_eq!(rope.len(), expected.len());

    let actual_summary = rope.summary();
    let expected_summary = TextSummary::from(expected.as_str());
    assert_eq!(actual_summary.len, expected_summary.len);
    assert_eq!(actual_summary.lines, expected_summary.lines);
    assert_eq!(actual_summary.chars, expected_summary.chars);
}

fn clip_offset(text: &str, mut offset: usize, bias: Bias) -> usize {
    while !text.is_char_boundary(offset) {
        match bias {
            Bias::Left => offset -= 1,
            Bias::Right => offset += 1,
        }
    }
    offset
}

impl Rope {
    fn text(&self) -> String {
        let mut text = String::new();
        for chunk in self.chunks.cursor::<()>(()) {
            text.push_str(&chunk.text);
        }
        text
    }
}

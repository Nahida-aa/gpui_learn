use crate::{OffsetUtf16, Point, PointUtf16, Unclipped};

use super::*;
use rand::prelude::*;
use sum_tree::Bias;
use util::RandomCharIter;

#[gpui::test(iterations = 100)]
fn test_random_chunks(mut rng: StdRng) {
    let text = random_string_with_utf8_len(&mut rng, MAX_BASE);
    log::info!("Chunk: {:?}", text);
    let chunk = Chunk::new(&text);
    verify_chunk(chunk.as_slice(), &text);

    // Verify Chunk::chars() bitmap
    let expected_chars = char_offsets(&text)
        .into_iter()
        .inspect(|i| assert!(*i < MAX_BASE))
        .fold(0 as Bitmap, |acc, i| acc | (1 << i));
    assert_eq!(chunk.chars(), expected_chars);

    for _ in 0..10 {
        let mut start = rng.random_range(0..=chunk.text.len());
        let mut end = rng.random_range(start..=chunk.text.len());
        while !chunk.text.is_char_boundary(start) {
            start -= 1;
        }
        while !chunk.text.is_char_boundary(end) {
            end -= 1;
        }
        let range = start..end;
        log::info!("Range: {:?}", range);
        let text_slice = &text[range.clone()];
        let chunk_slice = chunk.slice(range);
        verify_chunk(chunk_slice, text_slice);
    }
}

#[gpui::test(iterations = 100)]
fn test_split_chunk_slice(mut rng: StdRng) {
    let text = &random_string_with_utf8_len(&mut rng, MAX_BASE);
    let chunk = Chunk::new(text);
    let offset = char_offsets_with_end(text)
        .into_iter()
        .choose(&mut rng)
        .unwrap();
    let (a, b) = chunk.as_slice().split_at(offset);
    let (a_str, b_str) = text.split_at(offset);
    verify_chunk(a, a_str);
    verify_chunk(b, b_str);
}

#[gpui::test(iterations = 1000)]
fn test_nth_set_bit_random(mut rng: StdRng) {
    let set_count = rng.random_range(0..=128);
    let mut set_bits = (0..128).choose_multiple(&mut rng, set_count);
    set_bits.sort();
    let mut n = 0;
    for ix in set_bits.iter().copied() {
        n |= 1 << ix;
    }

    for (mut ix, position) in set_bits.into_iter().enumerate() {
        ix += 1;
        assert_eq!(
            nth_set_bit(n, ix),
            position,
            "nth_set_bit({:0128b}, {})",
            n,
            ix
        );
    }
}

/// Returns a (biased) random string whose UTF-8 length is no more than `len`.
fn random_string_with_utf8_len(rng: &mut StdRng, len: usize) -> String {
    let mut str = String::new();
    let mut chars = RandomCharIter::new(rng);
    loop {
        let ch = chars.next().unwrap();
        if str.len() + ch.len_utf8() > len {
            break;
        }
        str.push(ch);
    }
    str
}

#[gpui::test(iterations = 1000)]
fn test_append_random_strings(mut rng: StdRng) {
    let len1 = rng.random_range(0..=MAX_BASE);
    let len2 = rng.random_range(0..=MAX_BASE).saturating_sub(len1);
    let str1 = random_string_with_utf8_len(&mut rng, len1);
    let str2 = random_string_with_utf8_len(&mut rng, len2);
    let mut chunk1 = Chunk::new(&str1);
    let chunk2 = Chunk::new(&str2);
    let char_offsets = char_offsets_with_end(&str2);
    let start_index = rng.random_range(0..char_offsets.len());
    let start_offset = char_offsets[start_index];
    let end_offset = char_offsets[rng.random_range(start_index..char_offsets.len())];
    chunk1.append(chunk2.slice(start_offset..end_offset));
    verify_chunk(chunk1.as_slice(), &(str1 + &str2[start_offset..end_offset]));
}

#[gpui::test(iterations = 1000)]
fn test_prepend_random_strings(mut rng: StdRng) {
    let len1 = rng.random_range(0..=MAX_BASE);
    let len2 = rng.random_range(0..=MAX_BASE).saturating_sub(len1);
    let str1 = random_string_with_utf8_len(&mut rng, len1);
    let str2 = random_string_with_utf8_len(&mut rng, len2);
    let mut chunk1 = Chunk::new(&str1);
    let chunk2 = Chunk::new(&str2);
    let char_offsets = char_offsets_with_end(&str2);
    let start_index = rng.random_range(0..char_offsets.len());
    let start_offset = char_offsets[start_index];
    let end_offset = char_offsets[rng.random_range(start_index..char_offsets.len())];
    let slice = chunk2.slice(start_offset..end_offset);
    let prefix_text = &str2[start_offset..end_offset];
    chunk1.prepend(slice);
    verify_chunk(chunk1.as_slice(), &(prefix_text.to_owned() + &str1));
}

/// Return the byte offsets for each character in a string.
///
/// These are valid offsets to split the string.
fn char_offsets(text: &str) -> Vec<usize> { text.char_indices().map(|(i, _c)| i).collect() }

/// Return the byte offsets for each character in a string, plus the offset
/// past the end of the string.
fn char_offsets_with_end(text: &str) -> Vec<usize> {
    let mut v = char_offsets(text);
    v.push(text.len());
    v
}

fn verify_chunk(chunk: ChunkSlice<'_>, text: &str) {
    let mut offset = 0;
    let mut offset_utf16 = crate::OffsetUtf16(0);
    let mut point = crate::Point::zero();
    let mut point_utf16 = crate::PointUtf16::zero();

    log::info!("Verifying chunk {:?}", text);
    assert_eq!(chunk.offset_to_point(0), Point::zero());

    let mut expected_tab_positions = Vec::new();

    for (char_offset, c) in text.chars().enumerate() {
        let expected_point = chunk.offset_to_point(offset);
        assert_eq!(point, expected_point, "mismatch at offset {}", offset);
        assert_eq!(
            chunk.point_to_offset(point),
            offset,
            "mismatch at point {:?}",
            point
        );
        assert_eq!(
            chunk.offset_to_offset_utf16(offset),
            offset_utf16,
            "mismatch at offset {}",
            offset
        );
        assert_eq!(
            chunk.offset_utf16_to_offset(offset_utf16),
            offset,
            "mismatch at offset_utf16 {:?}",
            offset_utf16
        );
        assert_eq!(
            chunk.point_to_point_utf16(point),
            point_utf16,
            "mismatch at point {:?}",
            point
        );
        assert_eq!(
            chunk.point_utf16_to_offset(point_utf16, false),
            offset,
            "mismatch at point_utf16 {:?}",
            point_utf16
        );
        assert_eq!(
            chunk.unclipped_point_utf16_to_point(Unclipped(point_utf16)),
            point,
            "mismatch for unclipped_point_utf16_to_point at {:?}",
            point_utf16
        );

        assert_eq!(
            chunk.clip_point(point, Bias::Left),
            point,
            "incorrect left clip at {:?}",
            point
        );
        assert_eq!(
            chunk.clip_point(point, Bias::Right),
            point,
            "incorrect right clip at {:?}",
            point
        );

        for i in 1..c.len_utf8() {
            let test_point = Point::new(point.row, point.column + i as u32);
            assert_eq!(
                chunk.clip_point(test_point, Bias::Left),
                point,
                "incorrect left clip within multi-byte char at {:?}",
                test_point
            );
            assert_eq!(
                chunk.clip_point(test_point, Bias::Right),
                Point::new(point.row, point.column + c.len_utf8() as u32),
                "incorrect right clip within multi-byte char at {:?}",
                test_point
            );
        }

        for i in 1..c.len_utf16() {
            let test_point = Unclipped(PointUtf16::new(
                point_utf16.row,
                point_utf16.column + i as u32,
            ));
            assert_eq!(
                chunk.unclipped_point_utf16_to_point(test_point),
                point,
                "incorrect unclipped_point_utf16_to_point within multi-byte char at {:?}",
                test_point
            );
            assert_eq!(
                chunk.clip_point_utf16(test_point, Bias::Left),
                point_utf16,
                "incorrect left clip_point_utf16 within multi-byte char at {:?}",
                test_point
            );
            assert_eq!(
                chunk.clip_point_utf16(test_point, Bias::Right),
                PointUtf16::new(point_utf16.row, point_utf16.column + c.len_utf16() as u32),
                "incorrect right clip_point_utf16 within multi-byte char at {:?}",
                test_point
            );

            let test_offset = OffsetUtf16(offset_utf16.0 + i);
            assert_eq!(
                chunk.clip_offset_utf16(test_offset, Bias::Left),
                offset_utf16,
                "incorrect left clip_offset_utf16 within multi-byte char at {:?}",
                test_offset
            );
            assert_eq!(
                chunk.clip_offset_utf16(test_offset, Bias::Right),
                OffsetUtf16(offset_utf16.0 + c.len_utf16()),
                "incorrect right clip_offset_utf16 within multi-byte char at {:?}",
                test_offset
            );
        }

        if c == '\n' {
            point.row += 1;
            point.column = 0;
            point_utf16.row += 1;
            point_utf16.column = 0;
        } else {
            point.column += c.len_utf8() as u32;
            point_utf16.column += c.len_utf16() as u32;
        }

        if c == '\t' {
            expected_tab_positions.push(TabPosition {
                byte_offset: offset,
                char_offset,
            });
        }

        offset += c.len_utf8();
        offset_utf16.0 += c.len_utf16();
    }

    let final_point = chunk.offset_to_point(offset);
    assert_eq!(point, final_point, "mismatch at final offset {}", offset);
    assert_eq!(
        chunk.point_to_offset(point),
        offset,
        "mismatch at point {:?}",
        point
    );
    assert_eq!(
        chunk.offset_to_offset_utf16(offset),
        offset_utf16,
        "mismatch at offset {}",
        offset
    );
    assert_eq!(
        chunk.offset_utf16_to_offset(offset_utf16),
        offset,
        "mismatch at offset_utf16 {:?}",
        offset_utf16
    );
    assert_eq!(
        chunk.point_to_point_utf16(point),
        point_utf16,
        "mismatch at final point {:?}",
        point
    );
    assert_eq!(
        chunk.point_utf16_to_offset(point_utf16, false),
        offset,
        "mismatch at final point_utf16 {:?}",
        point_utf16
    );
    assert_eq!(
        chunk.unclipped_point_utf16_to_point(Unclipped(point_utf16)),
        point,
        "mismatch for unclipped_point_utf16_to_point at final point {:?}",
        point_utf16
    );
    assert_eq!(
        chunk.clip_point(point, Bias::Left),
        point,
        "incorrect left clip at final point {:?}",
        point
    );
    assert_eq!(
        chunk.clip_point(point, Bias::Right),
        point,
        "incorrect right clip at final point {:?}",
        point
    );
    assert_eq!(
        chunk.clip_point_utf16(Unclipped(point_utf16), Bias::Left),
        point_utf16,
        "incorrect left clip_point_utf16 at final point {:?}",
        point_utf16
    );
    assert_eq!(
        chunk.clip_point_utf16(Unclipped(point_utf16), Bias::Right),
        point_utf16,
        "incorrect right clip_point_utf16 at final point {:?}",
        point_utf16
    );
    assert_eq!(
        chunk.clip_offset_utf16(offset_utf16, Bias::Left),
        offset_utf16,
        "incorrect left clip_offset_utf16 at final offset {:?}",
        offset_utf16
    );
    assert_eq!(
        chunk.clip_offset_utf16(offset_utf16, Bias::Right),
        offset_utf16,
        "incorrect right clip_offset_utf16 at final offset {:?}",
        offset_utf16
    );

    // Verify length methods
    assert_eq!(chunk.len(), text.len());
    assert_eq!(
        chunk.len_utf16().0,
        text.chars().map(|c| c.len_utf16()).sum::<usize>()
    );

    // Verify line counting
    let lines = chunk.lines();
    let mut newline_count = 0;
    let mut last_line_len = 0;
    for c in text.chars() {
        if c == '\n' {
            newline_count += 1;
            last_line_len = 0;
        } else {
            last_line_len += c.len_utf8() as u32;
        }
    }
    assert_eq!(lines, Point::new(newline_count, last_line_len));

    // Verify first/last line chars
    if !text.is_empty() {
        let first_line = text.split('\n').next().unwrap();
        assert_eq!(chunk.first_line_chars(), first_line.chars().count() as u32);

        let last_line = text.split('\n').next_back().unwrap();
        assert_eq!(chunk.last_line_chars(), last_line.chars().count() as u32);
        assert_eq!(
            chunk.last_line_len_utf16(),
            last_line.chars().map(|c| c.len_utf16() as u32).sum::<u32>()
        );
    }

    // Verify longest row
    let (longest_row, longest_chars) = chunk.longest_row(&mut 0);
    let mut max_chars = 0;
    let mut current_row = 0;
    let mut current_chars = 0;
    let mut max_row = 0;

    for c in text.chars() {
        if c == '\n' {
            if current_chars > max_chars {
                max_chars = current_chars;
                max_row = current_row;
            }
            current_row += 1;
            current_chars = 0;
        } else {
            current_chars += 1;
        }
    }

    if current_chars > max_chars {
        max_chars = current_chars;
        max_row = current_row;
    }

    assert_eq!((max_row, max_chars as u32), (longest_row, longest_chars));
    assert_eq!(chunk.tabs().collect::<Vec<_>>(), expected_tab_positions);
}

#[gpui::test]
fn test_point_utf16_to_offset_clips_to_correct_absolute_offset() {
    let text = "abc\nde";
    let chunk = Chunk::new(text);
    let slice = chunk.as_slice();

    // Clipping on row 0 (row_offset_range.start == 0, so relative == absolute)
    assert_eq!(slice.point_utf16_to_offset(PointUtf16::new(0, 99), true), 3,);

    // Clipping on row 1 — this is the case that was buggy.
    // Row 1 starts at byte offset 4 ("de" is bytes 4..6), so the
    // clipped result must be 6, not 2.
    assert_eq!(slice.point_utf16_to_offset(PointUtf16::new(1, 99), true), 6,);
}

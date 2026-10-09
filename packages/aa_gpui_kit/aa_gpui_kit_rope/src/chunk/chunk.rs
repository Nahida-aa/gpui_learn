use super::*;
use crate::{OffsetUtf16, Point, PointUtf16, TextSummary, Unclipped};
use heapless::String as ArrayString;
use std::ops::Range;
use unicode_segmentation::GraphemeCursor;
use util::debug_panic;

#[derive(Clone, Debug, Default)]
pub struct Chunk {
    /// If bit[i] is set, then the byte at index i starts a UTF-8 character.
    pub(crate) chars: Bitmap,
    /// Parallel to `chars`; bit[i] and bit[i+1] are both set for characters
    /// that encode as two UTF-16 code units.
    pub(crate) chars_utf16: Bitmap,
    /// Bit[i] is set iff byte i is an ASCII newline.
    pub(crate) newlines: Bitmap,
    /// Bit[i] is set iff byte i is an ASCII tab.
    pub(crate) tabs: Bitmap,
    pub text: ArrayString<MAX_BASE, u8>,
}

impl Chunk {
    pub const MASK_BITS: usize = Bitmap::BITS as usize;

    #[inline(always)]
    pub fn new(text: &str) -> Self {
        let text = {
            let mut buf = ArrayString::new();
            buf.push_str(text).unwrap();
            buf
        };

        const CHUNK_SIZE: usize = 8;

        let mut chars_bytes = [0; MAX_BASE / CHUNK_SIZE];
        let mut newlines_bytes = [0; MAX_BASE / CHUNK_SIZE];
        let mut tabs_bytes = [0; MAX_BASE / CHUNK_SIZE];
        let mut chars_utf16_bytes = [0; MAX_BASE / CHUNK_SIZE];

        let mut chunk_ix = 0;

        let mut bytes = text.as_bytes();
        while !bytes.is_empty() {
            let (chunk, rest) = bytes.split_at(bytes.len().min(CHUNK_SIZE));
            bytes = rest;

            let mut chars = 0;
            let mut newlines = 0;
            let mut tabs = 0;
            let mut chars_utf16 = 0;

            for (ix, &b) in chunk.iter().enumerate() {
                chars |= (util::is_utf8_char_boundary(b) as u8) << ix;
                newlines |= ((b == b'\n') as u8) << ix;
                tabs |= ((b == b'\t') as u8) << ix;
                // b >= 240 when we are at the first byte of the 4 byte encoded
                // utf-8 code point (U+010000 or greater) it means that it would
                // be encoded as two 16-bit code units in utf-16
                chars_utf16 |= ((b >= 240) as u8) << ix;
            }

            chars_bytes[chunk_ix] = chars;
            newlines_bytes[chunk_ix] = newlines;
            tabs_bytes[chunk_ix] = tabs;
            chars_utf16_bytes[chunk_ix] = chars_utf16;

            chunk_ix += 1;
        }

        let chars = Bitmap::from_le_bytes(chars_bytes);

        Chunk {
            text,
            chars,
            chars_utf16: (Bitmap::from_le_bytes(chars_utf16_bytes) << 1) | chars,
            newlines: Bitmap::from_le_bytes(newlines_bytes),
            tabs: Bitmap::from_le_bytes(tabs_bytes),
        }
    }

    #[inline(always)]
    pub fn push_str(&mut self, text: &str) { self.append(Chunk::new(text).as_slice()); }

    #[inline(always)]
    pub fn prepend_str(&mut self, text: &str) { self.prepend(Chunk::new(text).as_slice()); }

    #[inline(always)]
    pub fn append(&mut self, slice: ChunkSlice) {
        if slice.is_empty() {
            return;
        };

        let base_ix = self.text.len();
        self.chars |= slice.chars << base_ix;
        self.chars_utf16 |= slice.chars_utf16 << base_ix;
        self.newlines |= slice.newlines << base_ix;
        self.tabs |= slice.tabs << base_ix;
        self.text.push_str(slice.text).unwrap();
    }

    #[inline(always)]
    pub fn prepend(&mut self, slice: ChunkSlice) {
        if slice.is_empty() {
            return;
        }
        if self.text.is_empty() {
            *self = Chunk::new(slice.text);
            return;
        }

        let shift = slice.text.len();
        self.chars = slice.chars | (self.chars << shift);
        self.chars_utf16 = slice.chars_utf16 | (self.chars_utf16 << shift);
        self.newlines = slice.newlines | (self.newlines << shift);
        self.tabs = slice.tabs | (self.tabs << shift);

        let mut new_text = ArrayString::<MAX_BASE, u8>::new();
        new_text.push_str(slice.text).unwrap();
        new_text.push_str(&self.text).unwrap();
        self.text = new_text;
    }

    #[inline(always)]
    pub fn as_slice(&self) -> ChunkSlice<'_> {
        ChunkSlice {
            chars: self.chars,
            chars_utf16: self.chars_utf16,
            newlines: self.newlines,
            tabs: self.tabs,
            text: &self.text,
        }
    }

    #[inline(always)]
    pub fn slice(&self, range: Range<usize>) -> ChunkSlice<'_> { self.as_slice().slice(range) }

    #[inline(always)]
    pub fn chars(&self) -> Bitmap { self.chars }

    #[inline(always)]
    pub fn tabs(&self) -> Bitmap { self.tabs }

    #[inline(always)]
    pub fn newlines(&self) -> Bitmap { self.newlines }

    #[inline(always)]
    pub fn is_char_boundary(&self, offset: usize) -> bool {
        (1 as Bitmap).unbounded_shl(offset as u32) & self.chars != 0 || offset == self.text.len()
    }

    pub fn floor_char_boundary(&self, index: usize) -> usize {
        if index >= self.text.len() {
            self.text.len()
        } else {
            let mut i = index;
            while i > 0 {
                if util::is_utf8_char_boundary(self.text.as_bytes()[i]) {
                    break;
                }
                i -= 1;
            }

            i
        }
    }

    #[track_caller]
    #[inline(always)]
    pub fn assert_char_boundary<const PANIC: bool>(&self, offset: usize) -> bool {
        if self.is_char_boundary(offset) {
            return true;
        }
        if PANIC || cfg!(debug_assertions) {
            panic_char_boundary(&self.text, offset);
        } else {
            log_err_char_boundary(&self.text, offset);
            false
        }
    }
}

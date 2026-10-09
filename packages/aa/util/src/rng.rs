//! `RandomCharIter` — a biased random char iterator, used by tests and test-support.

#![cfg(any(test, feature = "test-support"))]

use rand::prelude::*;

pub struct RandomCharIter<T: Rng> {
    rng: T,
    simple_text: bool,
}

impl<T: Rng> RandomCharIter<T> {
    pub fn new(rng: T) -> Self {
        Self {
            rng,
            simple_text: std::env::var("SIMPLE_TEXT").is_ok_and(|v| !v.is_empty()),
        }
    }

    pub fn with_simple_text(mut self) -> Self {
        self.simple_text = true;
        self
    }
}

impl<T: Rng> Iterator for RandomCharIter<T> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        if self.simple_text {
            return if self.rng.random_range(0..100) < 5 {
                Some('\n')
            } else {
                Some(self.rng.random_range(b'a'..b'z' + 1).into())
            };
        }

        match self.rng.random_range(0..100) {
            // whitespace
            0..=19 => [' ', '\n', '\r', '\t'].choose(&mut self.rng).copied(),
            // two-byte greek letters
            20..=32 => char::from_u32(self.rng.random_range(('α' as u32)..('ω' as u32 + 1))),
            // // three-byte characters
            33..=45 => ['✋', '✅', '❌', '❎', '⭐']
                .choose(&mut self.rng)
                .copied(),
            // // four-byte characters
            46..=58 => ['🍐', '🏀', '🍗', '🎉'].choose(&mut self.rng).copied(),
            // ascii letters
            _ => Some(self.rng.random_range(b'a'..b'z' + 1).into()),
        }
    }
}

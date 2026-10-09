use super::*;

pub struct Tabs {
    pub(crate) tabs: Bitmap,
    pub(crate) chars: Bitmap,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TabPosition {
    pub byte_offset: usize,
    pub char_offset: usize,
}

impl Iterator for Tabs {
    type Item = TabPosition;

    fn next(&mut self) -> Option<Self::Item> {
        if self.tabs == 0 {
            return None;
        }

        let tab_offset = self.tabs.trailing_zeros() as usize;
        let chars_mask = (1 << tab_offset) - 1;
        let char_offset = (self.chars & chars_mask).count_ones() as usize;

        // Since tabs are 1 byte the tab offset is the same as the byte offset
        let position = TabPosition {
            byte_offset: tab_offset,
            char_offset,
        };
        // Remove the tab we've just seen
        self.tabs ^= 1 << tab_offset;

        Some(position)
    }
}

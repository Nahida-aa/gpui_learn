
use rope::{Cursor, Rope};

use super::fragment::{Fragment, FragmentTextSummary};

pub(crate) struct RopeBuilder<'a> {
    old_visible_cursor: Cursor<'a>,
    old_deleted_cursor: Cursor<'a>,
    new_visible: Rope,
    new_deleted: Rope,
}

impl<'a> RopeBuilder<'a> {
    pub(crate) fn new(old_visible_cursor: Cursor<'a>, old_deleted_cursor: Cursor<'a>) -> Self {
        Self {
            old_visible_cursor,
            old_deleted_cursor,
            new_visible: Rope::new(),
            new_deleted: Rope::new(),
        }
    }

    pub(crate) fn append(&mut self, len: FragmentTextSummary) {
        self.push(len.visible, true, true);
        self.push(len.deleted, false, false);
    }

    pub(crate) fn push_fragment(&mut self, fragment: &Fragment, was_visible: bool) {
        debug_assert!(fragment.len > 0);
        self.push(fragment.len as usize, was_visible, fragment.visible)
    }

    fn push(&mut self, len: usize, was_visible: bool, is_visible: bool) {
        let text = if was_visible {
            self.old_visible_cursor
                .slice(self.old_visible_cursor.offset() + len)
        } else {
            self.old_deleted_cursor
                .slice(self.old_deleted_cursor.offset() + len)
        };
        if is_visible {
            self.new_visible.append(text);
        } else {
            self.new_deleted.append(text);
        }
    }

    pub(crate) fn push_str(&mut self, text: &str) { self.new_visible.push(text); }

    pub(crate) fn finish(mut self) -> (Rope, Rope) {
        self.new_visible.append(self.old_visible_cursor.suffix());
        self.new_deleted.append(self.old_deleted_cursor.suffix());
        (self.new_visible, self.new_deleted)
    }
}

use std::cmp;
use std::ops::{Range, Sub};

use sum_tree::{Bias, FilterCursor};

use crate::anchor::Anchor;
use crate::locator::Locator;
use crate::undo_map::UndoMap;
use rope::TextDimension;

use super::buffer_id::BufferId;
use super::fragment::{Fragment, FragmentSummary, FragmentTextSummary};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Edit<D> {
    pub old: Range<D>,
    pub new: Range<D>,
}

impl<D> Edit<D>
where
    D: PartialEq,
{
    pub fn is_empty(&self) -> bool {
        self.old.start == self.old.end && self.new.start == self.new.end
    }
}

impl<D, DDelta> Edit<D>
where
    D: Sub<D, Output = DDelta> + Copy,
{
    pub fn old_len(&self) -> DDelta { self.old.end - self.old.start }

    pub fn new_len(&self) -> DDelta { self.new.end - self.new.start }
}

impl<D1, D2> Edit<(D1, D2)> {
    pub fn flatten(self) -> (Edit<D1>, Edit<D2>) {
        (
            Edit {
                old: self.old.start.0..self.old.end.0,
                new: self.new.start.0..self.new.end.0,
            },
            Edit {
                old: self.old.start.1..self.old.end.1,
                new: self.new.start.1..self.new.end.1,
            },
        )
    }
}

pub(crate) struct Edits<'a, D: TextDimension, F: FnMut(&FragmentSummary) -> bool> {
    pub(crate) visible_cursor: rope::Cursor<'a>,
    pub(crate) deleted_cursor: rope::Cursor<'a>,
    pub(crate) fragments_cursor:
        Option<FilterCursor<'a, 'static, F, Fragment, FragmentTextSummary>>,
    pub(crate) undos: &'a UndoMap,
    pub(crate) since: &'a clock::Global,
    pub(crate) old_end: D,
    pub(crate) new_end: D,
    pub(crate) range: Range<(&'a Locator, u32)>,
    pub(crate) buffer_id: BufferId,
}

impl<D: TextDimension + Ord, F: FnMut(&FragmentSummary) -> bool> Iterator for Edits<'_, D, F> {
    type Item = (Edit<D>, Range<Anchor>);

    fn next(&mut self) -> Option<Self::Item> {
        let mut pending_edit: Option<Self::Item> = None;
        let cursor = self.fragments_cursor.as_mut()?;

        while let Some(fragment) = cursor.item() {
            if fragment.id < *self.range.start.0 {
                cursor.next();
                continue;
            } else if fragment.id > *self.range.end.0 {
                break;
            }

            if cursor.start().visible > self.visible_cursor.offset() {
                let summary = self.visible_cursor.summary(cursor.start().visible);
                self.old_end.add_assign(&summary);
                self.new_end.add_assign(&summary);
            }

            if pending_edit
                .as_ref()
                .is_some_and(|(change, _)| change.new.end < self.new_end)
            {
                break;
            }

            let start_anchor = Anchor::new(
                fragment.timestamp,
                fragment.insertion_offset,
                Bias::Right,
                self.buffer_id,
            );
            let end_anchor = Anchor::new(
                fragment.timestamp,
                fragment.insertion_offset + fragment.len,
                Bias::Left,
                self.buffer_id,
            );

            if !fragment.was_visible(self.since, self.undos) && fragment.visible {
                let mut visible_end = cursor.end().visible;
                if fragment.id == *self.range.end.0 {
                    visible_end = cmp::min(
                        visible_end,
                        cursor.start().visible
                            + (self.range.end.1 - fragment.insertion_offset) as usize,
                    );
                }

                let fragment_summary = self.visible_cursor.summary(visible_end);
                let mut new_end = self.new_end;
                new_end.add_assign(&fragment_summary);
                if let Some((edit, range)) = pending_edit.as_mut() {
                    edit.new.end = new_end;
                    range.end = end_anchor;
                } else {
                    pending_edit = Some((
                        Edit {
                            old: self.old_end..self.old_end,
                            new: self.new_end..new_end,
                        },
                        start_anchor..end_anchor,
                    ));
                }

                self.new_end = new_end;
            } else if fragment.was_visible(self.since, self.undos) && !fragment.visible {
                let mut deleted_end = cursor.end().deleted;
                if fragment.id == *self.range.end.0 {
                    deleted_end = cmp::min(
                        deleted_end,
                        cursor.start().deleted
                            + (self.range.end.1 - fragment.insertion_offset) as usize,
                    );
                }

                if cursor.start().deleted > self.deleted_cursor.offset() {
                    self.deleted_cursor.seek_forward(cursor.start().deleted);
                }
                let fragment_summary = self.deleted_cursor.summary(deleted_end);
                let mut old_end = self.old_end;
                old_end.add_assign(&fragment_summary);
                if let Some((edit, range)) = pending_edit.as_mut() {
                    edit.old.end = old_end;
                    range.end = end_anchor;
                } else {
                    pending_edit = Some((
                        Edit {
                            old: self.old_end..old_end,
                            new: self.new_end..self.new_end,
                        },
                        start_anchor..end_anchor,
                    ));
                }

                self.old_end = old_end;
            }

            cursor.next();
        }

        pending_edit
    }
}

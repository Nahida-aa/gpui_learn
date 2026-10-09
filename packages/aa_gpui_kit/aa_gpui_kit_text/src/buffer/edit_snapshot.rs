
use crate::buffer::BufferSnapshot;

pub struct EditedBufferSnapshot {
    pub base_version: clock::Global,
    pub snapshot: BufferSnapshot,
    pub did_edit: bool,
}

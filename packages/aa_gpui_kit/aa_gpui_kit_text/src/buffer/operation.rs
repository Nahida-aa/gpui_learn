use collections::HashMap;
use std::ops::{Range, Sub};
use std::sync::Arc;

use clock::{Lamport};

use super::dimensions::FullOffset;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Operation {
    Edit(EditOperation),
    Undo(UndoOperation),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditOperation {
    pub timestamp: Lamport,
    pub version: clock::Global,
    pub ranges: Vec<Range<FullOffset>>,
    pub new_text: Vec<Arc<str>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UndoOperation {
    pub timestamp: Lamport,
    pub version: clock::Global,
    pub counts: HashMap<Lamport, u32>,
}

impl Operation {
    pub(crate) fn replica_id(&self) -> clock::ReplicaId {
        crate::operation_queue::Operation::lamport_timestamp(self).replica_id
    }

    pub fn timestamp(&self) -> Lamport {
        match self {
            Operation::Edit(edit) => edit.timestamp,
            Operation::Undo(undo) => undo.timestamp,
        }
    }

    pub fn as_edit(&self) -> Option<&EditOperation> {
        match self {
            Operation::Edit(edit) => Some(edit),
            _ => None,
        }
    }

    pub fn is_edit(&self) -> bool { matches!(self, Operation::Edit { .. }) }
}

impl crate::operation_queue::Operation for Operation {
    fn lamport_timestamp(&self) -> Lamport {
        match self {
            Operation::Edit(edit) => edit.timestamp,
            Operation::Undo(undo) => undo.timestamp,
        }
    }
}

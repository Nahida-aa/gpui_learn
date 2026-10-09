use std::sync::Arc;
use std::time::Instant;

use gpui::SharedString;
use parking_lot::Mutex;

pub type JobId = usize;

#[derive(Clone, Debug)]
pub struct JobInfo {
    pub start: Instant,
    pub message: SharedString,
    pub id: JobId,
}

#[derive(Debug, Clone)]
pub enum JobEvent {
    Started { info: JobInfo },
    Updated { id: JobId, message: SharedString },
    Completed { id: JobId },
}

pub type JobEventSender = futures::channel::mpsc::UnboundedSender<JobEvent>;
pub type JobEventReceiver = futures::channel::mpsc::UnboundedReceiver<JobEvent>;

pub(crate) struct JobTracker {
    pub(crate) id: JobId,
    pub(crate) subscribers: Arc<Mutex<Vec<JobEventSender>>>,
}

impl JobTracker {
    pub(crate) fn new(info: JobInfo, subscribers: Arc<Mutex<Vec<JobEventSender>>>) -> Self {
        let id = info.id;
        {
            let mut subs = subscribers.lock();
            subs.retain(|sender| {
                sender
                    .unbounded_send(JobEvent::Started { info: info.clone() })
                    .is_ok()
            });
        }
        Self { id, subscribers }
    }

    pub(crate) fn update(&self, message: SharedString) {
        let mut subscribers = self.subscribers.lock();
        subscribers.retain(|sender| {
            sender
                .unbounded_send(JobEvent::Updated {
                    id: self.id,
                    message: message.clone(),
                })
                .is_ok()
        });
    }
}

impl Drop for JobTracker {
    fn drop(&mut self) {
        let mut subs = self.subscribers.lock();
        subs.retain(|sender| {
            sender
                .unbounded_send(JobEvent::Completed { id: self.id })
                .is_ok()
        });
    }
}

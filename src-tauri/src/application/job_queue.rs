use std::collections::VecDeque;

use crate::domain::job::{Job, JobStatus};

#[derive(Debug)]
pub struct JobQueue {
    concurrency_limit: usize,
    active: usize,
    pending: VecDeque<Job>,
}

impl JobQueue {
    pub fn new(concurrency_limit: usize) -> Self {
        assert!(concurrency_limit > 0, "concurrency limit must be positive");
        Self {
            concurrency_limit,
            active: 0,
            pending: VecDeque::new(),
        }
    }

    pub fn enqueue(&mut self, job: Job) {
        self.pending.push_back(job);
    }

    pub fn take_next(&mut self) -> Option<Job> {
        if self.active >= self.concurrency_limit {
            return None;
        }

        self.pending.pop_front().map(|mut job| {
            job.status = JobStatus::Running;
            self.active += 1;
            job
        })
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let original_len = self.pending.len();
        self.pending.retain(|job| job.id != id);
        self.pending.len() != original_len
    }

    pub fn finish_active(&mut self) {
        self.active = self.active.saturating_sub(1);
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::job::AudioFormat;

    fn job(id: &str) -> Job {
        Job::queued_audio(id, "input.wav", "output.mp3", AudioFormat::Mp3, 0)
    }

    #[test]
    fn preserves_fifo_order_and_respects_the_concurrency_limit() {
        let mut queue = JobQueue::new(1);
        queue.enqueue(job("first"));
        queue.enqueue(job("second"));

        assert_eq!(queue.pending_count(), 2);
        assert_eq!(queue.take_next().map(|job| job.id), Some("first".into()));
        assert!(queue.take_next().is_none());

        queue.finish_active();
        assert_eq!(queue.take_next().map(|job| job.id), Some("second".into()));
    }

    #[test]
    fn queued_jobs_can_be_removed_before_execution() {
        let mut queue = JobQueue::new(1);
        queue.enqueue(job("cancel-me"));

        assert!(queue.remove("cancel-me"));
        assert_eq!(queue.pending_count(), 0);
    }
}

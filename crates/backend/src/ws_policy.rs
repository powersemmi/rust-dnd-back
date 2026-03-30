use axum::extract::ws::{CloseFrame, Utf8Bytes, close_code};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub(crate) const MAX_INBOUND_MESSAGE_SIZE_BYTES: usize = 1024 * 1024;

const GENERAL_MESSAGE_LIMIT: usize = 60;
const MOUSE_MESSAGE_LIMIT: usize = 1200;
const FILE_CHUNK_MESSAGE_LIMIT: usize = 1200;
const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IncomingMessageKind {
    General,
    Mouse,
    FileChunk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RateLimitViolation {
    General,
    Mouse,
    FileChunk,
}

#[derive(Debug)]
pub(crate) struct ConnectionRateLimiter {
    general: SlidingWindow,
    mouse: SlidingWindow,
    file_chunk: SlidingWindow,
}

impl Default for ConnectionRateLimiter {
    fn default() -> Self {
        Self {
            general: SlidingWindow::new(GENERAL_MESSAGE_LIMIT, RATE_LIMIT_WINDOW),
            mouse: SlidingWindow::new(MOUSE_MESSAGE_LIMIT, RATE_LIMIT_WINDOW),
            file_chunk: SlidingWindow::new(FILE_CHUNK_MESSAGE_LIMIT, RATE_LIMIT_WINDOW),
        }
    }
}

impl ConnectionRateLimiter {
    pub(crate) fn check(&mut self, kind: IncomingMessageKind) -> Result<(), RateLimitViolation> {
        self.check_at(kind, Instant::now())
    }

    fn check_at(
        &mut self,
        kind: IncomingMessageKind,
        now: Instant,
    ) -> Result<(), RateLimitViolation> {
        let (window, violation) = match kind {
            IncomingMessageKind::General => (&mut self.general, RateLimitViolation::General),
            IncomingMessageKind::Mouse => (&mut self.mouse, RateLimitViolation::Mouse),
            IncomingMessageKind::FileChunk => (&mut self.file_chunk, RateLimitViolation::FileChunk),
        };

        if window.try_record(now) {
            Ok(())
        } else {
            Err(violation)
        }
    }
}

pub(crate) fn close_frame_for_violation(violation: RateLimitViolation) -> CloseFrame {
    let reason = match violation {
        RateLimitViolation::General => "Too many messages in a 10s window",
        RateLimitViolation::Mouse => "Too many mouse events in a 10s window",
        RateLimitViolation::FileChunk => "Too many file chunks in a 10s window",
    };

    CloseFrame {
        code: close_code::POLICY,
        reason: Utf8Bytes::from(reason),
    }
}

#[derive(Debug)]
struct SlidingWindow {
    limit: usize,
    window: Duration,
    entries: VecDeque<Instant>,
}

impl SlidingWindow {
    fn new(limit: usize, window: Duration) -> Self {
        Self {
            limit,
            window,
            entries: VecDeque::with_capacity(limit),
        }
    }

    fn try_record(&mut self, now: Instant) -> bool {
        while let Some(oldest) = self.entries.front() {
            if now.duration_since(*oldest) < self.window {
                break;
            }
            self.entries.pop_front();
        }

        if self.entries.len() >= self.limit {
            return false;
        }

        self.entries.push_back(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ConnectionRateLimiter, IncomingMessageKind, RATE_LIMIT_WINDOW, RateLimitViolation,
        close_frame_for_violation,
    };
    use axum::extract::ws::close_code;
    use std::time::{Duration, Instant};

    #[test]
    fn general_messages_use_sliding_window() {
        let mut limiter = ConnectionRateLimiter::default();
        let start = Instant::now();

        for index in 0..60 {
            let now = start + Duration::from_millis(index);
            assert_eq!(limiter.check_at(IncomingMessageKind::General, now), Ok(()));
        }

        let violation =
            limiter.check_at(IncomingMessageKind::General, start + Duration::from_secs(1));
        assert_eq!(violation, Err(RateLimitViolation::General));

        let recovered = limiter.check_at(
            IncomingMessageKind::General,
            start + RATE_LIMIT_WINDOW + Duration::from_millis(1),
        );
        assert_eq!(recovered, Ok(()));
    }

    #[test]
    fn mouse_messages_have_separate_budget() {
        let mut limiter = ConnectionRateLimiter::default();
        let start = Instant::now();

        for index in 0..1200 {
            let now = start + Duration::from_micros(index as u64);
            assert_eq!(limiter.check_at(IncomingMessageKind::Mouse, now), Ok(()));
        }

        let violation =
            limiter.check_at(IncomingMessageKind::Mouse, start + Duration::from_secs(1));
        assert_eq!(violation, Err(RateLimitViolation::Mouse));
    }

    #[test]
    fn close_frame_uses_policy_violation_code() {
        let close_frame = close_frame_for_violation(RateLimitViolation::General);

        assert_eq!(close_frame.code, close_code::POLICY);
        assert_eq!(close_frame.reason, "Too many messages in a 10s window");
    }

    #[test]
    fn file_chunks_have_separate_budget() {
        let mut limiter = ConnectionRateLimiter::default();
        let start = Instant::now();

        for index in 0..1200 {
            let now = start + Duration::from_millis(index);
            assert_eq!(
                limiter.check_at(IncomingMessageKind::FileChunk, now),
                Ok(())
            );
        }

        let violation = limiter.check_at(
            IncomingMessageKind::FileChunk,
            start + Duration::from_secs(1),
        );
        assert_eq!(violation, Err(RateLimitViolation::FileChunk));
    }
}

//! Multi-frame stability: only accept a payload that repeats identically.
//!
//! A single frame can decode to a stale or partially captured QR (a scrolling
//! stream, a half-drawn window). Requiring `need` identical observations
//! removes that class of false positive.
//!
//! "Identical" tolerates a few **undecodable** frames in between: a QR with an
//! animated overlay (the game client's rotating "已过期，点击刷新" icon) decodes
//! on one frame and fails on the next, and a strict consecutive rule then never
//! fires — the caller waits forever with nothing to show the operator. Missing
//! frames are counted with [`StabilityFilter::miss`]; the candidate survives up
//! to [`StabilityFilter::MISS_TOLERANCE`] of them.

/// Tracks identical payloads, tolerating a few failed frames between hits.
#[derive(Debug, Clone)]
pub struct StabilityFilter {
    need: usize,
    last: Option<String>,
    hits: usize,
    misses: usize,
}

impl StabilityFilter {
    /// How many failed frames may interrupt a candidate before it is dropped.
    pub const MISS_TOLERANCE: usize = 3;

    /// `need` is clamped to at least 1.
    pub fn new(need: usize) -> Self {
        Self {
            need: need.max(1),
            last: None,
            hits: 0,
            misses: 0,
        }
    }

    /// How many identical observations are required.
    pub fn required(&self) -> usize {
        self.need
    }

    /// Feed one decoded observation. Returns `true` once the payload has been
    /// seen `need` times (with at most `MISS_TOLERANCE` undecodable frames in
    /// between).
    pub fn observe(&mut self, payload: &str) -> bool {
        if self.last.as_deref() == Some(payload) {
            self.hits += 1;
        } else {
            self.last = Some(payload.to_string());
            self.hits = 1;
        }
        self.misses = 0;
        self.hits >= self.need
    }

    /// Feed one **undecodable** frame. Keeps the candidate alive for a few
    /// frames, then drops it so a stale payload cannot linger forever.
    pub fn miss(&mut self) {
        self.misses += 1;
        if self.misses > Self::MISS_TOLERANCE {
            self.reset();
        }
    }

    /// Forget the current candidate (e.g. when switching sources).
    pub fn reset(&mut self) {
        self.last = None;
        self.hits = 0;
        self.misses = 0;
    }

    /// Observations accumulated for the current candidate.
    pub fn hits(&self) -> usize {
        self.hits
    }

    /// Undecodable frames since the last hit.
    pub fn misses(&self) -> usize {
        self.misses
    }

    pub fn candidate(&self) -> Option<&str> {
        self.last.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_observation_is_enough_when_need_is_one() {
        let mut f = StabilityFilter::new(1);
        assert!(f.observe("A"));
    }

    #[test]
    fn needs_two_identical_frames() {
        let mut f = StabilityFilter::new(2);
        assert!(!f.observe("A"));
        assert!(f.observe("A"));
    }

    #[test]
    fn a_different_payload_restarts_the_count() {
        let mut f = StabilityFilter::new(2);
        assert!(!f.observe("A"));
        assert!(!f.observe("B"));
        assert!(f.observe("B"));
        assert_eq!(f.candidate(), Some("B"));
        assert_eq!(f.hits(), 2);
    }

    #[test]
    fn reset_clears_the_candidate() {
        let mut f = StabilityFilter::new(2);
        f.observe("A");
        f.reset();
        assert!(!f.observe("A"));
        assert_eq!(f.hits(), 1);
    }

    #[test]
    fn need_zero_is_clamped_to_one() {
        let mut f = StabilityFilter::new(0);
        assert_eq!(f.required(), 1);
        assert!(f.observe("A"));
    }

    /// 过期二维码带旋转图标：解码一帧成功、一帧失败，也必须能判稳。
    #[test]
    fn undecodable_frames_do_not_starve_a_candidate() {
        let mut f = StabilityFilter::new(2);
        assert!(!f.observe("A"));
        f.miss();
        assert!(f.observe("A"));
        assert_eq!(f.misses(), 0);
    }

    /// 但连续失败太多就作废：陈旧的码不能一直挂在候选里。
    #[test]
    fn too_many_misses_drop_the_candidate() {
        let mut f = StabilityFilter::new(2);
        assert!(!f.observe("A"));
        for _ in 0..StabilityFilter::MISS_TOLERANCE {
            f.miss();
        }
        assert_eq!(f.candidate(), Some("A"));
        f.miss();
        assert_eq!(f.candidate(), None);
        assert!(!f.observe("A"));
    }
}

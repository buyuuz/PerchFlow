use std::time::Duration;

pub struct Backoff {
    initial: u64,
    max: u64,
    attempt: u32,
}

impl Backoff {
    pub fn new(initial_secs: u64, max_secs: u64) -> Self {
        Self { initial: initial_secs.max(1), max: max_secs, attempt: 0 }
    }

    pub fn next_delay(&mut self) -> Duration {
        let secs = self.initial.saturating_mul(1u64 << self.attempt.min(20));
        self.attempt = self.attempt.saturating_add(1);
        Duration::from_secs(secs.min(self.max))
    }

    pub fn reset(&mut self) {
        self.attempt = 0;
    }

    pub fn attempts(&self) -> u32 {
        self.attempt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doubles_each_time() {
        let mut b = Backoff::new(2, 60);
        assert_eq!(b.next_delay(), Duration::from_secs(2));
        assert_eq!(b.next_delay(), Duration::from_secs(4));
        assert_eq!(b.next_delay(), Duration::from_secs(8));
    }

    #[test]
    fn caps_at_max() {
        let mut b = Backoff::new(10, 15);
        assert_eq!(b.next_delay(), Duration::from_secs(10));
        assert_eq!(b.next_delay(), Duration::from_secs(15));
        assert_eq!(b.next_delay(), Duration::from_secs(15));
    }

    #[test]
    fn reset_restores_initial() {
        let mut b = Backoff::new(2, 60);
        b.next_delay();
        b.next_delay();
        b.reset();
        assert_eq!(b.attempts(), 0);
        assert_eq!(b.next_delay(), Duration::from_secs(2));
    }
}

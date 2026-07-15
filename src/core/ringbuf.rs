use std::collections::VecDeque;

pub struct RingBuffer {
    capacity: usize,
    buf: VecDeque<String>,
}

impl RingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self { capacity, buf: VecDeque::with_capacity(capacity) }
    }

    pub fn push(&mut self, line: String) {
        if self.buf.len() == self.capacity {
            self.buf.pop_front();
        }
        self.buf.push_back(line);
    }

    pub fn recent(&self, n: usize) -> Vec<String> {
        let start = self.buf.len().saturating_sub(n);
        self.buf.iter().skip(start).cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_recent_within_capacity() {
        let mut rb = RingBuffer::new(3);
        rb.push("a".into());
        rb.push("b".into());
        assert_eq!(rb.recent(10), vec!["a", "b"]);
    }

    #[test]
    fn evicts_oldest_when_full() {
        let mut rb = RingBuffer::new(2);
        rb.push("a".into());
        rb.push("b".into());
        rb.push("c".into());
        assert_eq!(rb.recent(10), vec!["b", "c"]);
        assert_eq!(rb.len(), 2);
    }

    #[test]
    fn recent_returns_last_n() {
        let mut rb = RingBuffer::new(5);
        for s in ["a", "b", "c", "d"] { rb.push(s.into()); }
        assert_eq!(rb.recent(2), vec!["c", "d"]);
    }
}


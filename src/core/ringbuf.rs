use std::collections::VecDeque;

pub struct RingBuffer {
    capacity: usize,
    buf: VecDeque<(u64, String)>,
    next_seq: u64,
}

impl RingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self { capacity, buf: VecDeque::with_capacity(capacity), next_seq: 0 }
    }

    pub fn push(&mut self, line: String) {
        if self.buf.len() == self.capacity {
            self.buf.pop_front();
        }
        self.buf.push_back((self.next_seq, line));
        self.next_seq += 1;
    }

    pub fn recent(&self, n: usize) -> Vec<String> {
        let start = self.buf.len().saturating_sub(n);
        self.buf.iter().skip(start).map(|(_, s)| s.clone()).collect()
    }

    /// 返回最近 n 条带全局递增序号 (seq, line) 的日志。seq 从 0 开始单调递增,
    /// 供前端按序号做增量渲染(清屏后只显示新日志)。
    pub fn recent_with_seq(&self, n: usize) -> Vec<(u64, String)> {
        let start = self.buf.len().saturating_sub(n);
        self.buf.iter().skip(start).cloned().collect()
    }

    /// 下一条日志将被分配的序号,等于已 push 的日志总数。
    /// 空 buffer 时为 0。前端用它做"已渲染到哪"的基线。
    pub fn next_seq(&self) -> u64 {
        self.next_seq
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


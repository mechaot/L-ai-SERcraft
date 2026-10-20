//! Port of `Tools/MobileAverageCalculator.cs`: `MobileAverageCalculator` (long) and `MobileDAverageCalculator` (double).

/// Integer moving average (exponential-style window).
#[derive(Debug, Clone, Default)]
pub struct MobileAverageCalculator {
    size: i32,
    filled_size: i32,
    sum: i64,
    cur: i64,
    max: i64,
    min: i64,
    avg: i64,
    ready: bool,
}

impl MobileAverageCalculator {
    /// New calculator.
    ///
    /// `size`: window size (must be > 0).
    pub fn new(size: i32) -> Self {
        Self { size, ..Default::default() }
    }

    /// New calculator seeded with a first sample.
    ///
    /// `size`: window size. `init`: first sample.
    pub fn with_init(size: i32, init: i64) -> Self {
        let mut c = Self::new(size);
        c.enqueue_new_sample(init);
        c
    }

    /// Clear all state.
    pub fn reset(&mut self) {
        *self = Self { size: self.size, ..Default::default() };
    }

    /// Add a sample.
    ///
    /// `value`: new sample.
    pub fn enqueue_new_sample(&mut self, value: i64) {
        let size = self.size as i64;
        if self.ready {
            self.sum = self.sum - self.sum / size + value;
        } else {
            self.sum = value * size;
            self.ready = true;
        }
        if self.filled_size < self.size {
            self.filled_size += 1;
        }
        self.cur = value;
        // NOTE(upstream): max/min start at 0, so min never goes above 0
        self.max = self.max.max(self.cur);
        self.min = self.min.min(self.cur);
        self.avg = self.sum / size;
    }

    /// Change window size keeping the current average.
    ///
    /// `new_size`: new window.
    pub fn resize(&mut self, new_size: i32) {
        self.sum = self.avg * new_size as i64;
        self.size = new_size;
    }

    /// Last sample.
    pub fn current_value(&self) -> i64 {
        self.cur
    }
    /// Max seen (starts at 0).
    pub fn max(&self) -> i64 {
        self.max
    }
    /// Min seen (starts at 0).
    pub fn min(&self) -> i64 {
        self.min
    }
    /// Average.
    pub fn avg(&self) -> i64 {
        self.avg
    }
    /// True after the first sample.
    pub fn ready(&self) -> bool {
        self.ready
    }
    /// True when window is full.
    pub fn filled(&self) -> bool {
        self.filled_size == self.size
    }
}

/// Floating point moving average.
#[derive(Debug, Clone, Default)]
pub struct MobileDAverageCalculator {
    size: i32,
    sum: f64,
    cur: f64,
    max: f64,
    min: f64,
    avg: f64,
    ready: bool,
}

impl MobileDAverageCalculator {
    /// New calculator.
    ///
    /// `size`: window size.
    pub fn new(size: i32) -> Self {
        Self { size, ..Default::default() }
    }

    /// New calculator seeded with a first sample.
    ///
    /// `size`: window size. `init`: first sample.
    pub fn with_init(size: i32, init: f64) -> Self {
        let mut c = Self::new(size);
        c.enqueue_new_sample(init);
        c
    }

    /// Clear and seed.
    ///
    /// `init`: first sample.
    pub fn reset_with(&mut self, init: f64) {
        self.reset();
        self.enqueue_new_sample(init);
    }

    /// Clear all state.
    pub fn reset(&mut self) {
        *self = Self { size: self.size, ..Default::default() };
    }

    /// Add a sample.
    ///
    /// `value`: new sample.
    pub fn enqueue_new_sample(&mut self, value: f64) {
        let size = self.size as f64;
        if self.ready {
            self.sum = self.sum - self.sum / size + value;
        } else {
            self.sum = value * size;
            self.ready = true;
        }
        self.cur = value;
        self.max = self.max.max(self.cur);
        self.min = self.min.min(self.cur);
        self.avg = self.sum / size;
    }

    /// Change window size keeping the current average.
    ///
    /// `new_size`: new window.
    pub fn resize(&mut self, new_size: i32) {
        self.sum = self.avg * new_size as f64;
        self.size = new_size;
    }

    /// Last sample.
    pub fn current_value(&self) -> f64 {
        self.cur
    }
    /// Max seen (starts at 0).
    pub fn max(&self) -> f64 {
        self.max
    }
    /// Min seen (starts at 0).
    pub fn min(&self) -> f64 {
        self.min
    }
    /// Average.
    pub fn avg(&self) -> f64 {
        self.avg
    }
    /// True after the first sample.
    pub fn ready(&self) -> bool {
        self.ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_avg() {
        let mut m = MobileAverageCalculator::with_init(4, 100);
        assert!(m.ready());
        assert_eq!(m.avg(), 100);
        assert!(!m.filled());
        m.enqueue_new_sample(200); // sum 400-100+200=500
        assert_eq!(m.avg(), 125);
        for _ in 0..3 {
            m.enqueue_new_sample(0);
        }
        assert!(m.filled());
        assert_eq!(m.max(), 200);
        assert_eq!(m.min(), 0);
        m.reset();
        assert!(!m.ready());
    }

    #[test]
    fn double_avg() {
        let mut m = MobileDAverageCalculator::with_init(2, 10.0);
        m.enqueue_new_sample(20.0); // 20-10+20 = 30
        assert_eq!(m.avg(), 15.0);
        m.resize(4);
        assert_eq!(m.avg(), 15.0);
    }
}

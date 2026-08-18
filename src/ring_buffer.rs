//! Real-Time безопасный кольцевой буфер с нулевыми аллокациями в аудиопотоке.

pub struct FixedRingBuffer {
    buffer: Vec<f32>,
    capacity: usize,
    read_pos: usize,
    write_pos: usize,
    available: usize,
}

impl FixedRingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0.0; capacity],
            capacity,
            read_pos: 0,
            write_pos: 0,
            available: 0,
        }
    }

    pub fn clear(&mut self) {
        self.read_pos = 0;
        self.write_pos = 0;
        self.available = 0;
        self.buffer.fill(0.0);
    }

    #[inline(always)]
    pub fn push_slice(&mut self, data: &[f32]) {
        for &sample in data {
            if self.available < self.capacity {
                self.buffer[self.write_pos] = sample;
                self.write_pos = (self.write_pos + 1) % self.capacity;
                self.available += 1;
            }
        }
    }

    #[inline(always)]
    pub fn read_chunk(&mut self, out: &mut [f32]) -> bool {
        if self.available < out.len() {
            return false;
        }
        for sample in out.iter_mut() {
            *sample = self.buffer[self.read_pos];
            self.read_pos = (self.read_pos + 1) % self.capacity;
        }
        self.available -= out.len();
        true
    }

    #[inline(always)]
    pub fn available_samples(&self) -> usize {
        self.available
    }
}
use wasmtime::*;

pub struct MemoryLimiter {
    linear_memory_limit_in_bytes: usize,
}

impl MemoryLimiter {
    pub fn new(linear_memory_limit_in_bytes: usize) -> Self {
        Self {
            linear_memory_limit_in_bytes,
        }
    }
}

impl ResourceLimiter for MemoryLimiter {
    fn memory_growing(&mut self, current: usize, _desired: usize, _maximum: Option<usize>) -> bool {
        if current > self.linear_memory_limit_in_bytes {
            return false;
        }

        true
    }

    fn table_growing(&mut self, _current: u32, _desired: u32, _maximum: Option<u32>) -> bool {
        true
    }
}

use std::ptr;
use time;

#[derive(Copy)]
pub struct Buffer {
    pub start: i64,
    pub end: i64,
    data: [u64; 60],
}

/// Buffer is a counter buffer with a max size of 60,
/// intended to internally hold counts for 60 successive
/// time units (seconds).
impl Buffer {
    pub fn new() -> Buffer {
        let start = time::get_time().sec;
        Buffer::start_at(start)
    }

    pub fn start_at(start: i64) -> Buffer {
        let end = start + 59;
        Buffer {
            start: start,
            end: end,
            data: [0; 60],
        }
    }

    #[inline(always)]
    pub fn contains(&self, index: i64) -> bool {
        index >= self.start && index <= self.end
    }

    /// Unsafe because it skips bounds checking
    pub unsafe fn incr(&mut self, index: i64) {
        let i = (index - self.start) as usize;
        let elem = self.data.get_unchecked_mut(i);
        *elem += 1;
    }

    pub fn reset_at(&mut self, index: i64) {
        unsafe {
            let vec_ptr = self.data.as_mut_ptr();
            ptr::write_bytes(vec_ptr, 0, self.data.len());
        }
        self.start = index;
        self.end = index + 59;
    }
}

impl Clone for Buffer {
    fn clone(&self) -> Buffer {
        let mut data = [0; 60];
        data.copy_from_slice(&self.data);
        Buffer {
            start: self.start,
            end: self.end,
            data: data,
        }
    }
}

impl<'a> IntoIterator for &'a Buffer {
    type Item = &'a u64;
    type IntoIter = ::std::slice::Iter<'a, u64>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}
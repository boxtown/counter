extern crate time;
extern crate fnv;

use std::io::{self, Write};
use std::fs::{File, OpenOptions};
use std::ptr;
use fnv::FnvHashMap;

/// Enumeration of possible Counter-related errors
pub enum Error {
    /// Occurs when buffers are full and must be flushed to storage
    MustFlushCounter,
    /// Occurs when an IO-related error occurs during Counter operations
    IO(io::Error),
}

/// Custom Counter Result type that always returns a Counter::Error
/// as the error type
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Copy)]
struct Buffer {
    start: i64,
    end: i64,
    data: [u64; 60],
}

/// Buffer is a counter buffer with a max size of 60,
/// intended to internally hold counts for 60 successive
/// time units (seconds).
impl Buffer {
    fn new() -> Buffer {
        let start = time::get_time().sec;
        let end = start + 59;
        Buffer {
            start: start,
            end: end,
            data: [0; 60],
        }
    }

    #[inline(always)]
    fn contains(&self, index: i64) -> bool {
        index >= self.start && index <= self.end
    }

    fn incr(&mut self, index: i64) {
        let i = (index - self.start) as usize;
        unsafe {
            let elem = self.data.get_unchecked_mut(i);
            *elem += 1;
        }
    }

    fn reset_at(&mut self, index: i64) {
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

/// Trait for buffer writers
trait WriteBuffer {
    fn write(&mut self, buf: &Buffer) -> Result<()>;
}

/// Trait for Counter data writers
trait WriteCounterData {
    fn write(&mut self, data: &FnvHashMap<String, Buffer>) -> Result<()>;
}

/// Default writer for the counter that writes data
/// to a file
struct CounterFileWriter {
    file: File,
    current_day: i32,
}

impl CounterFileWriter {
    fn new() -> Result<CounterFileWriter> {
        let now = time::now_utc();
        let path = CounterFileWriter::gen_path(&now);

        let mut write_header = false;
        let mut file = try!(CounterFileWriter::open_append(&path).or_else(|_| {
            write_header = true;
            CounterFileWriter::open_new(&path)
        }));
        if write_header {
            let today = CounterFileWriter::get_day_start(&now).to_timespec();
            try!(file.write_all(&format!("{};", today.sec).into_bytes()).map_err(Error::IO));
        }

        Ok(CounterFileWriter {
            file: file,
            current_day: now.tm_yday,
        })
    }

    /// Opens a file at path for appending. File
    /// must exist
    fn open_append(path: &str) -> Result<File> {
        OpenOptions::new()
            .append(true)
            .open(path)
            .map_err(Error::IO)
    }

    /// Opens a new file at path. File must not exist
    fn open_new(path: &str) -> Result<File> {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(Error::IO)
    }

    /// Generate a file path name for given the time
    fn gen_path(tm: &time::Tm) -> String {
        format!("counter-{}-{}.dat", tm.tm_yday, 1900 + tm.tm_year)
    }

    /// Return a time at 00:00:00 of the same day
    /// as the given time
    fn get_day_start(now: &time::Tm) -> time::Tm {
        time::Tm {
            tm_sec: 0,
            tm_min: 0,
            tm_hour: 0,
            tm_mday: now.tm_mday,
            tm_mon: now.tm_mon,
            tm_year: now.tm_year,
            tm_wday: now.tm_wday,
            tm_yday: now.tm_yday,
            tm_isdst: now.tm_isdst,
            tm_utcoff: now.tm_utcoff,
            tm_nsec: 0,
        }
    }
}

impl WriteCounterData for CounterFileWriter {
    fn write(&mut self, data: &FnvHashMap<String, Buffer>) -> Result<()> {

        Ok(())
    }
}

pub struct Counter {
    data: FnvHashMap<String, Buffer>,
    writer: CounterFileWriter,
}

impl Counter {
    pub fn new() -> Result<Counter> {
        let writer = try!(CounterFileWriter::new());
        Ok(Counter {
            data: FnvHashMap::default(),
            writer: writer,
        })
    }

    pub fn incr(&mut self, key: &str) -> Result<()> {
        let bi = time::get_time().sec;
        let mut buf = self.data.entry(key.to_owned()).or_insert(Buffer::new());
        if buf.contains(bi) {
            buf.incr(bi);
            Ok(())
        } else {
            Err(Error::MustFlushCounter)
        }
    }

    fn flush(&mut self, start: i64) -> Result<()> {
        try!(self.writer.write(&self.data));
        for buf in self.data.values_mut() {
            buf.reset_at(start);
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {}
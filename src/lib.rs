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
pub struct Buffer {
    start: i64,
    end: i64,
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

/// Type for counter data map
pub type DataMap = FnvHashMap<String, Buffer>;

/// Trait for Counter data writers
pub trait WriteCounterData {
    fn write(&mut self, data: &DataMap) -> Result<()>;
}

/// Default writer for the counter that writes data
/// to a file
pub struct CounterFileWriter {
    file: File,
    day_start: time::Tm,
}

impl CounterFileWriter {
    pub fn new() -> Result<CounterFileWriter> {
        let (file, today) = try!(CounterFileWriter::open_todays_file());
        Ok(CounterFileWriter {
            file: file,
            day_start: today,
        })
    }

    fn open_todays_file() -> Result<(File, time::Tm)> {
        let day_start = CounterFileWriter::get_day_start(&time::now_utc());
        let path = CounterFileWriter::gen_path(&day_start);

        let mut write_header = false;
        let mut file = try!(CounterFileWriter::open_append(&path).or_else(|_| {
            write_header = true;
            CounterFileWriter::open_new(&path)
        }));
        if write_header {
            try!(file.write_all(&format!("{};", day_start.to_timespec().sec).into_bytes())
                .map_err(Error::IO));
        }
        Ok((file, day_start))
    }


    fn is_new_day(&self) -> bool {
        self.day_start.tm_yday != time::now_utc().tm_yday
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

    fn write_buffer(offset: i64, buf: &Buffer, dest: &mut String) {
        for (i, &val) in buf.into_iter().enumerate() {
            if val == 0 {
                continue;
            }
            dest.push_str(&format!("{}", buf.start + i as i64 - offset));
            dest.push(':');
            dest.push_str(&format!("{}", val));
            dest.push(',');
        }
        dest.pop();
    }

    fn write_data_to_string(offset: i64, data: &DataMap) -> String {
        let mut s_buf = String::with_capacity(CounterFileWriter::guess_size(data));
        for (key, buf) in data {
            s_buf.push('{');
            s_buf.push_str(key);
            s_buf.push(';');
            CounterFileWriter::write_buffer(offset, buf, &mut s_buf);
            s_buf.push('}');
        }
        s_buf
    }

    fn guess_size(data: &DataMap) -> usize {
        data.iter().fold(0, |acc, pair| {
            let (key, buf) = pair;
            acc + key.len() + 3 +
            buf.into_iter().fold(0, |acc, &x| {
                acc +
                match x {
                    0 => 0,
                    _ => 16,
                }
            })
        })
    }
}

// NOTE: figure out how to write to same string buffer from WriteBuffer
impl WriteCounterData for CounterFileWriter {
    fn write(&mut self, data: &DataMap) -> Result<()> {
        if self.is_new_day() {
            let (file, day_start) = try!(CounterFileWriter::open_todays_file());
            self.file = file;
            self.day_start = day_start;
        }

        let offset = self.day_start.to_timespec().sec;
        let result = CounterFileWriter::write_data_to_string(offset, data);
        try!(self.file.write_all(result.as_bytes()).map_err(Error::IO));
        Ok(())
    }
}

pub struct Counter<T>
    where T: WriteCounterData
{
    data: DataMap,
    writer: T,
}

impl<T> Counter<T>
    where T: WriteCounterData
{
    pub fn new() -> Result<Counter<CounterFileWriter>> {
        let writer = try!(CounterFileWriter::new());
        Ok(Counter {
            data: DataMap::default(),
            writer: writer,
        })
    }

    pub fn with_writer(writer: T) -> Counter<T> {
        Counter {
            data: DataMap::default(),
            writer: writer,
        }
    }

    pub fn incr(&mut self, key: &str) -> Result<()> {
        let bi = time::get_time().sec;
        let mut buf = self.data.entry(key.to_owned()).or_insert(Buffer::new());
        if buf.contains(bi) {
            unsafe {
                buf.incr(bi);
            }
            Ok(())
        } else {
            Err(Error::MustFlushCounter)
        }
    }

    pub fn flush(&mut self, start: i64) -> Result<()> {
        try!(self.writer.write(&self.data));
        for buf in self.data.values_mut() {
            buf.reset_at(start);
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_guess_size() {
        let mut data = DataMap::default();
        data.insert("test1".to_owned(), Buffer::start_at(0));
        data.insert("test2".to_owned(), Buffer::start_at(0));
        unsafe {
            data.get_mut("test1").unwrap().incr(0);
            data.get_mut("test2").unwrap().incr(1);
            data.get_mut("test2").unwrap().incr(2);
        }
        let size = CounterFileWriter::guess_size(&data);
        assert_eq!(size, 64);
    }

    #[test]
    fn test_write_buffer() {
        let mut buf = Buffer::start_at(1500);
        unsafe {
            buf.incr(1505);
            buf.incr(1505);
            buf.incr(1532);
            buf.incr(1516);
            buf.incr(1516);
            buf.incr(1516);
        }
        let mut s_buf = String::new();
        CounterFileWriter::write_buffer(200, &buf, &mut s_buf);
        assert_eq!(s_buf, "1305:2,1316:3,1332:1")
    }

    #[test]
    fn test_write_data_to_string() {
        let mut data = DataMap::default();
        data.insert("test1".to_owned(), Buffer::start_at(1500));
        data.insert("test2".to_owned(), Buffer::start_at(1500));
        unsafe {
            data.get_mut("test1").unwrap().incr(1500);
            data.get_mut("test2").unwrap().incr(1501);
            data.get_mut("test2").unwrap().incr(1502);
        }
        let result = CounterFileWriter::write_data_to_string(200, &data);
        assert_eq!(result, "{test1;1300:1}{test2;1301:1,1302:1}");
    }
}
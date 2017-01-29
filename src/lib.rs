extern crate time;
extern crate fnv;

pub mod buffer;
pub mod writer;

use std::io;
use fnv::FnvHashMap;
use buffer::Buffer;
use writer::CounterFileWriter;

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

/// Type for counter data map
pub type DataMap = FnvHashMap<String, buffer::Buffer>;

/// Trait for Counter data writers
pub trait WriteCounterData {
    fn write(&mut self, data: &DataMap) -> Result<()>;
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
                buf.incr(bi);// Opens a file at path for appending. File
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
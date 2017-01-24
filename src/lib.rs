extern crate time;

use std::ptr;
use std::io::{Result, Write};
use std::fs::{File, OpenOptions};

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

trait WriteBuffer {
    fn write(&mut self, buf: &Buffer) -> Result<()>;
}

trait WriteCounterData {
    fn write(&mut self, data: &[Buffer]) -> Result<()>;
}

struct CounterFileWriter {
    file: File,
    current_day: i32,
}

impl CounterFileWriter {
    fn new() -> Result<CounterFileWriter> {
        let now = time::now_utc();
        let path = CounterFileWriter::gen_path(&now);

        let mut write_header = false;
        let mut file = try!(OpenOptions::new()
            .append(true)
            .open(&path)
            .or_else(|_| {
                write_header = true;
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)
            }));
        if write_header {
            let today = CounterFileWriter::get_day(&now).to_timespec();
            try!(file.write_all(&format!("{};", today.sec).into_bytes()));
        }

        Ok(CounterFileWriter {
            file: file,
            current_day: now.tm_yday,
        })
    }

    fn gen_path(tm: &time::Tm) -> String {
        format!("counter-{}-{}.dat", tm.tm_yday, 1900 + tm.tm_year)
    }

    fn get_day(now: &time::Tm) -> time::Tm {
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
    fn write(&mut self, data: &[Buffer]) -> Result<()> {
        Ok(())
    }
}

pub struct Counter {
    data: [Buffer; 35 * 256],
    writer: CounterFileWriter,
}

impl Counter {
    pub fn new() -> Result<Counter> {
        let writer = try!(CounterFileWriter::new());
        Ok(Counter {
            data: [Buffer::new(); 35 * 256],
            writer: writer,
        })
    }

    pub fn incr(&mut self, key: &str) -> Result<()> {
        let i = hash(key);
        let bi = time::get_time().sec;
        if self.data[i].contains(bi) {
            self.data[i].incr(bi);
        } else {
            try!(self.flush(bi));
            self.data[i].incr(bi);
        }
        Ok(())
    }

    fn flush(&mut self, start: i64) -> Result<()> {
        try!(self.writer.write(&self.data));
        for buf in self.data.iter_mut() {
            buf.reset_at(start);
        }
        Ok(())
    }
}

// Hashes a string by summing the translated values of its characters
fn hash(key: &str) -> usize {
    key.chars().fold(0, |acc, c| acc + char_value(c))
}

// The value of a valid counter key character
fn char_value(c: char) -> usize {
    match c {
        '0' => 0,
        '1' => 1,
        '2' => 2,
        '3' => 3,
        '4' => 4,
        '5' => 5,
        '6' => 6,
        '7' => 7,
        '8' => 8,
        '9' => 9,
        'a' | 'A' => 10,
        'b' | 'B' => 11,
        'c' | 'C' => 12,
        'd' | 'D' => 13,
        'e' | 'E' => 14,
        'f' | 'F' => 15,
        'g' | 'G' => 16,
        'h' | 'H' => 17,
        'i' | 'I' => 18,
        'j' | 'J' => 19,
        'k' | 'K' => 20,
        'l' | 'L' => 21,
        'm' | 'M' => 22,
        'n' | 'N' => 23,
        'o' | 'O' => 24,
        'p' | 'P' => 25,
        'q' | 'Q' => 26,
        'r' | 'R' => 27,
        's' | 'S' => 28,
        't' | 'T' => 29,
        'u' | 'U' => 30,
        'v' | 'V' => 31,
        'w' | 'W' => 32,
        'x' | 'X' => 33,
        'y' | 'Y' => 34,
        'z' | 'Z' => 35,
        _ => panic!("Unsupported character {}", c),
    }
}

#[cfg(test)]
mod test {}
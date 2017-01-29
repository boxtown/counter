use std::io::Write;
use std::fs::{File, OpenOptions};
use time;
use super::{Result, Error, DataMap, WriteCounterData};
use super::buffer::Buffer;

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

#[cfg(test)]
mod test {
    use super::CounterFileWriter;
    use buffer::Buffer;
    use DataMap;

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
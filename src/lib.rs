extern crate time;
extern crate fnv;

pub mod bit_vec;

use bit_vec::AppendOnlyBitVec;

struct Last {
    delta: i64,
    ts: i64,
}

struct Deltas {
    delta: i64,
    delta_delta: i64,
}

pub struct TimeSeries {
    last: Option<Last>,
    data: AppendOnlyBitVec,
}

impl TimeSeries {
    pub fn new() -> TimeSeries {
        let ts = time::get_time().sec;
        let mut data = AppendOnlyBitVec::with_capacity(128);
        data.append(64, ts as u64); // set timestamp block
        TimeSeries {
            last: None,
            data: data,
        }
    }

    pub fn header(&self) -> u64 {
        self.data.get_block(0)
    }

    pub fn publish(&mut self, value: f64) {
        let compress_value = self.append_timestamp();

        // do value compression
        if !compress_value {
            self.data.append(64, value as u64);
        }
    }

    // Reference: http://www.vldb.org/pvldb/vol8/p1816-teller.pdf Section 4.1.1 Compressing time stamps
    fn append_timestamp(&mut self) -> bool {
        let ts = time::get_time().sec;
        if let Some(ref mut last) = self.last {
            let Deltas { delta, delta_delta } = TimeSeries::calculate_deltas(last, ts);
            last.ts = ts;
            last.delta = delta;
            match delta_delta {
                0 => self.data.append(1, 0),
                -63...64 => {
                    self.data.append(2, 0b10);
                    self.data.append(7, delta_delta as u64);
                }
                -255...256 => {
                    self.data.append(3, 0b110);
                    self.data.append(9, delta_delta as u64);
                }
                -2047...2048 => {
                    self.data.append(4, 0b1110);
                    self.data.append(12, delta_delta as u64);
                }
                _ => {
                    self.data.append(4, 0b1111);
                    self.data.append(32, delta_delta as u64);
                }
            }
            true
        } else {
            let last = Last {
                ts: ts,
                delta: ts - self.header() as i64,
            };
            self.data.append(14, last.delta as u64);
            self.last = Some(last);
            false
        }
    }

    fn calculate_deltas(last: &Last, ts: i64) -> Deltas {
        let delta = ts - last.ts;
        Deltas {
            delta: delta,
            delta_delta: delta - last.delta,
        }
    }
}
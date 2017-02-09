extern crate time;
extern crate fnv;

pub mod bit_vec;

use bit_vec::AppendOnlyBitVec;

struct Last {
    delta: i64,
    ts: i64,
    val: u64,
    leading: u32,
    trailing: u32,
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

    // Reference: http://www.vldb.org/pvldb/vol8/p1816-teller.pdf
    pub fn publish(&mut self, value: f64) {
        let ts = time::get_time().sec;
        if let Some(ref mut last) = self.last {
            // timestamp compression
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

            // value compression
            let v_u64 = value as u64;
            let xor = v_u64 ^ last.val;
            if xor == 0 {
                self.data.append(1, 0);
            } else {
                let leading = xor.leading_zeros();
                let trailing = xor.trailing_zeros();
                if last.leading <= leading && last.trailing <= trailing {
                    self.data.append(2, 0b10); // control bits
                    self.data.append((64 - last.leading - last.trailing) as usize,
                                     xor >> last.trailing); // meaningful bits
                } else {
                    self.data.append(1, 1); // control bit
                    self.data.append(5, leading as u64); // # leading zeros

                    let n_meaningful = 64 - leading - trailing;
                    self.data.append(6, n_meaningful as u64); // length of meaninful section in bits
                    self.data.append(n_meaningful as usize, xor >> trailing); // meaningful bits
                    last.val = v_u64;
                    last.leading = leading;
                    last.trailing = trailing;
                }
            }
        } else {
            // first value is uncompressed
            let v_u64 = value as u64;
            let last = Last {
                ts: ts,
                delta: ts - self.header() as i64,
                val: v_u64,
                leading: v_u64.leading_zeros(),
                trailing: v_u64.trailing_zeros(),
            };
            self.data.append(14, last.delta as u64);
            self.data.append(64, last.val);
            self.last = Some(last);
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
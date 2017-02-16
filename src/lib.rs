extern crate time;
extern crate fnv;

pub mod bit_vec;

use std::mem;
use bit_vec::AppendOnlyBitVec;

// contains data point information from the last
// data point that was published
struct Last {
    delta: i64,
    ts: i64,
    val: u64,
    leading: u32,
    trailing: u32,
}

// Contains calculated timestamp delta information
struct Deltas {
    delta: i64,
    delta_delta: i64,
}

// Contains compressed block info
struct CompressedBlock {
    bits: usize,
    block: u64,
}

pub struct TimeSeriesBlock {
    last: Option<Last>,
    data: AppendOnlyBitVec,
}

impl TimeSeriesBlock {
    pub fn new() -> TimeSeriesBlock {
        let ts = time::get_time().sec;
        TimeSeriesBlock::at(ts)
    }

    pub fn at(ts: i64) -> TimeSeriesBlock {
        let mut data = AppendOnlyBitVec::with_capacity(1024);
        data.append(64, ts as u64);
        TimeSeriesBlock {
            last: None,
            data: data,
        }
    }

    pub fn header(&self) -> u64 {
        self.data.get_block(0)
    }

    pub fn publish(&mut self, value: f64) {
        let ts = time::get_time().sec;
        self.publish_at(value, ts);
    }

    // Reference: http://www.vldb.org/pvldb/vol8/p1816-teller.pdf
    pub fn publish_at(&mut self, value: f64, ts: i64) {
        if let Some(ref mut last) = self.last {
            // timestamp compression
            let Deltas { delta, delta_delta } = TimeSeriesBlock::calculate_deltas(last, ts);
            last.ts = ts;
            last.delta = delta;
            let CompressedBlock { bits, block } =
                TimeSeriesBlock::compressed_time_block(delta_delta);
            self.data.append(bits, block);

            // value compression
            let v_u64 = unsafe { mem::transmute::<f64, u64>(value) };
            let xor = v_u64 ^ last.val;
            let leading = xor.leading_zeros();
            let trailing = xor.trailing_zeros();
            if xor == 0 {
                self.data.append(1, 0);
            } else {
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
                }
            }
            last.val = v_u64;
            last.leading = leading;
            last.trailing = trailing;
        } else {
            // first value is uncompressed
            let v_u64 = unsafe { mem::transmute::<f64, u64>(value) };
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

    fn compressed_time_block(dd: i64) -> CompressedBlock {
        match dd {
            0 => {
                CompressedBlock {
                    bits: 1,
                    block: 0,
                }
            }
            -63...64 => {
                let masked_value = dd as u64 & (!0 >> 57);
                CompressedBlock {
                    bits: 9,
                    block: (0b10 << 7) | masked_value,
                }
            }
            -255...256 => {
                let masked_value = dd as u64 & (!0 >> 55);
                CompressedBlock {
                    bits: 12,
                    block: (0b110 << 9) | masked_value,
                }
            }
            -2047...2048 => {
                let masked_value = dd as u64 & (!0 >> 52);
                CompressedBlock {
                    bits: 16,
                    block: (0b1110 << 12) | masked_value,
                }
            }
            _ => {
                let masked_value = dd as u64 & (!0 >> 32);
                CompressedBlock {
                    bits: 36,
                    block: (0b1111 << 32) | masked_value,
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::TimeSeriesBlock;

    #[test]
    fn test_publish() {
        let mut ts = TimeSeriesBlock::at(0);
        ts.publish_at(2.0, 5);
        ts.publish_at(4.0, 10);
        ts.publish_at(4.0, 20);
        ts.publish_at(2.0, 25);
        assert_eq!([0u64,
                    0b0000000000010101000000000000000000000000000000000000000000000000,
                    0b0000000000000001010110000011100000101010111101110101100000110000,
                    0],
                   ts.data.data());
    }
}
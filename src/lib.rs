extern crate time;
extern crate fnv;

pub mod bit_vec;

use std::mem;
use bit_vec::AppendOnlyBitVec;

/// Time Series data block (holds 2 hours of data with second precision)
pub struct TSBlock {
    last: Option<Last>,
    data: AppendOnlyBitVec,
}

impl TSBlock {
    /// Creates a new TSBlock starting at the current time
    pub fn new() -> TSBlock {
        let ts = time::get_time().sec;
        TSBlock::at(ts)
    }

    /// Creates a new TSBlock starting at `ts` seconds from the epoch
    pub fn at(ts: i64) -> TSBlock {
        let mut data = AppendOnlyBitVec::with_capacity(1024);
        data.append(64, ts as u64);
        TSBlock {
            last: None,
            data: data,
        }
    }

    /// Retrieves the timestamp header for the block
    /// (64-bit integer representing seconds since epoch)
    pub fn header(&self) -> u64 {
        self.data.get_block(0)
    }

    /// Publish a value to the time block at the current time
    pub fn publish(&mut self, value: f64) {
        let ts = time::get_time().sec;
        self.publish_at(value, ts);
    }

    /// Publish a value to the time block at the given `ts` seconds from the epoch
    /// Reference: http://www.vldb.org/pvldb/vol8/p1816-teller.pdf
    pub fn publish_at(&mut self, value: f64, ts: i64) {
        if let Some(ref mut last) = self.last {
            // timestamp compression
            let Deltas { delta, delta_delta } = TSBlock::calculate_deltas(last, ts);
            last.ts = ts;
            last.delta = delta;
            let CompressedBlock { bits, block } = TSBlock::compressed_time_block(delta_delta);
            self.data.append(bits, block);

            // value compression
            let v_u64 = unsafe { mem::transmute::<f64, u64>(value) };
            let xor = v_u64 ^ last.val;
            let leading = xor.leading_zeros();
            let trailing = xor.trailing_zeros();
            let zinfo = ZeroInfo {
                leading: leading,
                trailing: trailing,
                last_leading: last.leading,
                last_trailing: last.trailing,
            };
            let CompressedBlock { bits, block } = TSBlock::compressed_value_block(xor, zinfo);
            self.data.append(bits, block);
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
                // 1 bit if delta delta is 0
                CompressedBlock {
                    bits: 1,
                    block: 0,
                }
            }
            -63...64 => {
                // block construction:
                // 2 bits for header (left shift 7)
                // 7 bits for value
                let masked_value = dd as u64 & (!0 >> 57);
                CompressedBlock {
                    bits: 9,
                    block: (0b10 << 7) | masked_value,
                }
            }
            -255...256 => {
                // block construction:
                // 3 bits for header (left shift 9)
                // 9 bits for value
                let masked_value = dd as u64 & (!0 >> 55);
                CompressedBlock {
                    bits: 12,
                    block: (0b110 << 9) | masked_value,
                }
            }
            -2047...2048 => {
                // block construction:
                // 4 bits for header (left shift 12)
                // 12 bits for value
                let masked_value = dd as u64 & (!0 >> 52);
                CompressedBlock {
                    bits: 16,
                    block: (0b1110 << 12) | masked_value,
                }
            }
            _ => {
                // block construction:
                // 4 bits for header (left shift 32)
                // 32 bits for value
                let masked_value = dd as u64 & (!0 >> 32);
                CompressedBlock {
                    bits: 36,
                    block: (0b1111 << 32) | masked_value,
                }
            }
        }
    }

    fn compressed_value_block(xor: u64, zinfo: ZeroInfo) -> CompressedBlock {
        if xor == 0 {
            // 1 bit if xor is 0
            return CompressedBlock {
                bits: 1,
                block: 0,
            };
        }
        if zinfo.last_leading <= zinfo.leading && zinfo.last_trailing <= zinfo.trailing {
            // block construction:
            // 2 controls bits (left shift n)
            // n bits of meaningful section
            let len = 64 - zinfo.last_leading - zinfo.last_trailing;
            let control = 0b10 << len; // control bits
            let meaningful = xor >> zinfo.last_trailing; // meaningful section
            CompressedBlock {
                bits: 2 + len as usize,
                block: control | meaningful,
            }
        } else {
            // block construction:
            // 1 control bit (left shift 5 + 6 + n)
            // 5 bits for # of leading zeros (left shift 6 + n)
            // 6 bits for length of meaningful section  (left shift n)
            // n bits of meaningful section
            let len = 64 - zinfo.leading - zinfo.trailing;
            let control = 1u64 << (11 + len); // control bit
            let leading = (zinfo.leading as u64) << (6 + len); // # leading zeros
            let n_meaningful = (len as u64) << len; // length of meaninful section
            let meaningful = xor >> zinfo.trailing as usize; // meaningful section
            CompressedBlock {
                bits: 12 + len as usize,
                block: control | leading | n_meaningful | meaningful,
            }
        }
    }
}

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

// Contains info on leading and trailing zeros.
// Used to calculate the compressed value block
struct ZeroInfo {
    leading: u32,
    trailing: u32,
    last_leading: u32,
    last_trailing: u32,
}

#[cfg(test)]
mod test {
    use super::TSBlock;

    #[test]
    fn test_publish() {
        let mut ts = TSBlock::at(0);
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
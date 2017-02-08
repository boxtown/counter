use std::ptr;

pub struct AppendOnlyBitVec {
    vec: BitVec,
    len: usize,
}

impl AppendOnlyBitVec {
    pub fn new() -> AppendOnlyBitVec {
        AppendOnlyBitVec {
            vec: BitVec::new(),
            len: 0,
        }
    }

    pub fn with_capacity(nbits: usize) -> AppendOnlyBitVec {
        AppendOnlyBitVec {
            vec: BitVec::with_capacity(nbits),
            len: 0,
        }
    }

    pub fn get_bit(&self, index: usize) -> bool {
        self.vec.get_bit(index)
    }

    pub fn get_block(&self, index: usize) -> u64 {
        self.vec.get_block(index)
    }

    pub fn append(&mut self, bits: usize, data: u64) {
        match bits {
            0 => {}
            1 => {
                self.vec.set_bit(self.len, data & 1 == 1);
                self.len += 1;
            }
            64 => {
                self.vec.set_block(self.len, data);
                self.len += 64;
            }
            _ => {
                let mask = !0 >> (64 - bits as u64);
                let data = (data & mask) << (63 - bits);
                self.vec.set_block(self.len, data);
                self.len += bits;
            }
        }
    }
}

pub struct BitVec {
    data: Vec<u64>,
}

impl BitVec {
    pub fn new() -> BitVec {
        BitVec { data: Vec::new() }
    }

    pub fn with_capacity(nbits: usize) -> BitVec {
        BitVec { data: Vec::with_capacity(blocks(nbits)) }
    }

    pub fn clear(&mut self) {
        unsafe {
            let vec_ptr = self.data.as_mut_ptr();
            ptr::write_bytes(vec_ptr, 0, self.data.len());
        }
    }

    pub fn get_bit(&self, index: usize) -> bool {
        if self.out_of_bounds(index) {
            return false;
        }

        let block = unsafe { self.block(index) };
        let offset = offset_i(index);
        let mask = 1 << offset;
        (block & mask) >> offset == 1
    }

    pub fn set_bit(&mut self, index: usize, value: bool) {
        if self.out_of_bounds(index) {
            self.data.resize(blocks(index + 1), 0);
        }

        let mut block = unsafe { self.block_mut(index) };
        let offset = offset_i(index);
        let mask = 1 << offset;
        if value {
            *block |= mask;
        } else {
            *block &= !mask;
        }
    }

    pub fn get_block(&self, index: usize) -> u64 {
        // Algorithm example:
        //
        // Let index = 64 (block aligned)
        //
        // Blocks:
        //
        // |--------||--------|
        // |   0    ||   1    |
        // |--------||--------|
        //                ^
        //                |
        //             block (64 / 64 == 1)
        //
        // Block 1:
        //
        // |----------------------------------------------------------------|
        // |xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx|
        // |----------------------------------------------------------------|
        //  ^
        //  |
        // offset (canonical index: 0, actual bit index: 63)
        // 63 - 64 % 64 = 63
        //
        // Since offset is block aligned we simply return the block
        //
        // Let index = 67 (not block aligned)
        //
        // Block index is the same as when index = 64
        //
        // Block 1:
        //
        // |----------------------------------------------------------------|
        // |xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx|
        // |----------------------------------------------------------------|
        //     ^
        //     |
        //  offset (canonical index: 3, actual bit index: 60)
        //  63 - 67 % 64 = 60
        //
        // Step 1: Shift current block 3 (63 - offset) bits to the left
        // Step 2: OR shifted block with empty result block
        //
        // Let block 1 be:
        //
        // 10101100 11110000 00000000 11111111 00000000 000000000 00000000 11111111
        //
        // Result block after OR:
        //
        // 01100111 10000000 00000111 11111999 00000000 00000000 00000111 11111000
        //
        // Step 3: Grab next block
        // Step 4: Calculate amount of desired bits (64 - (offset + 1) == 3)
        // Step 5: Right shift next block 61 bits (offset + 1)
        // Step 6: OR result
        //
        // Let next block be:
        //
        // 01011000 10001111 00001111 11110000 00000000 1111111 11111111 00000000
        //
        // Block after shifting:
        // 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000010
        //
        // Returned block:
        //
        // 01100111 10000000 00000111 11111999 00000000 00000000 00000111 11111010
        if block_i(index) >= self.data.len() {
            // returns 0 if out of bounds
            return 0;
        }
        let block = unsafe { self.block(index) };
        let offset = offset_i(index);
        if offset == 63 {
            return *block;
        }
        let result = 0 | (*block << (63 - offset));
        if block_i(index) + 1 >= self.data.len() {
            // return early if out of bounds
            return result;
        }
        let next_block = unsafe { self.next_block(index) };
        result | (*next_block >> (offset + 1))
    }

    pub fn set_block(&mut self, index: usize, block: u64) {
        if self.out_of_bounds(index + 63) {
            self.data.resize(blocks(index + 64), 0);
        }

        // Algorithm example:
        //
        // Let index = 67
        //
        // Blocks after resize:
        //
        // |--------||--------||--------|
        // |    0   ||    1   ||   2    |
        // |--------||--------||--------|
        //                ^
        //                |
        //            cur_block (67 / 64 == 1)
        //
        // Block 1:
        //
        // |----------------------------------------------------------------|
        // |0000000000000000000000000000000000000000000000000000000000000000|
        // |----------------------------------------------------------------|
        //     ^
        //     |
        //   cur_offset (canonical index: 3, actual bit index: 6-)
        //   63 - 67 % 64 = 60. We do 63 - x so that bit indexes start
        //   at bit 63 and work to bit 0 so that blocks are contiguous
        //
        // Let input block be:
        //
        // 00000000 11111111 11111111 00000000 11111111 11111111 01011011 00000001
        //
        // Block 1 mask:
        //
        // 11100000 00000000 00000000 00000000 00000000 00000000 00000000 00000000
        //      ^
        //      |
        //   !0 << 61 == !0 << offset_i(67) + 1
        //
        // Block 1 data:
        //
        // 00000000 00011111 11111111 11100000 00011111 11111111 11101011 01100000
        //      ^
        //      |
        //   block >> 3 == block >> 64 - (offset_i(67) + 1)
        //
        // Block 1 after masking and setting
        //
        // xxx00000 00000011 11111111 11111100 00000011 11111111 11111101 01101100
        //
        // where xxx represent pre-existing bits
        //
        // Block 2 mask:
        //
        // 00011111 11111111 11111111 11111111 11111111 11111111 11111111 11111111
        //      ^
        //      |
        //   !0 >> 64 - (offset_i(67) + 1)
        //
        // Block 2 data:
        //
        // 00100000 00000000 00000000 00000000 00000000 00000000 00000000 00000000
        //      ^
        //      |
        //   block << 61 == block << offset_i(67) + 1
        if !self.set_cur_block(index, block) {
            self.set_next_block(index, block);
        }
    }

    // Sets 64 - block_offset bits in the block indicated by index at
    // block_offset. Returns true if index is blocked aligned, false otherwise
    fn set_cur_block(&mut self, index: usize, block: u64) -> bool {
        let mut cur_block = unsafe { self.block_mut(index) };
        if block_aligned(index) {
            *cur_block = block;
            true
        } else {
            let offset = offset_i(index);
            let mask = !0 << (offset + 1);
            let data = block >> (64 - (offset + 1));
            *cur_block = (*cur_block & mask) | data;
            false
        }
    }

    // Sets 64 - (64 - block_offset) bits at index 0 of the block
    // after the block indicated by index
    fn set_next_block(&mut self, index: usize, block: u64) {
        let mut cur_block = unsafe { self.next_block_mut(index) };
        let offset = offset_i(index);
        let mask = !0 >> (64 - (offset + 1));
        let data = block << (offset + 1);
        *cur_block = (*cur_block & mask) | data;
    }

    // Retrieves a reference to the block for the given index
    unsafe fn block(&self, index: usize) -> &u64 {
        self.data.get_unchecked(block_i(index))
    }

    // Retrieves a mutable reference to the block for the given index
    unsafe fn block_mut(&mut self, index: usize) -> &mut u64 {
        self.data.get_unchecked_mut(block_i(index))
    }

    // Retrives a reference to the block after the index block
    unsafe fn next_block(&self, index: usize) -> &u64 {
        self.data.get_unchecked(block_i(index) + 1)
    }

    // Retrieves a mutable reference to the block after the index block
    unsafe fn next_block_mut(&mut self, index: usize) -> &mut u64 {
        self.data.get_unchecked_mut(block_i(index) + 1)
    }

    // Returns if the index resides in a block that is out of bounds
    // of the current data vector
    fn out_of_bounds(&self, index: usize) -> bool {
        blocks(index + 1) > self.data.len()
    }
}

/// Returns the 0-based index of the block given the index
fn block_i(index: usize) -> usize {
    index / 64
}

/// Returns the 0-based block offset for index
fn offset_i(index: usize) -> u64 {
    63 - index as u64 % 64
}

fn block_aligned(index: usize) -> bool {
    index % 64 == 0
}

/// Returns the number of 32 bit blocks it takes to contain
/// nbits
fn blocks(nbits: usize) -> usize {
    match nbits % 64 {
        0 => nbits >> 6,
        _ => (nbits >> 6) + 1,
    }
}

#[cfg(test)]
mod test {
    use super::{AppendOnlyBitVec, BitVec};

    #[test]
    fn test_get_set() {
        let mut vec = BitVec::new();
        vec.set_bit(0, true);
        vec.set_bit(5, true);
        vec.set_bit(10, true);
        assert_eq!(true, vec.get_bit(0));
        assert_eq!(true, vec.get_bit(5));
        assert_eq!(true, vec.get_bit(10));
        assert_eq!(false, vec.get_bit(32));
        vec.set_bit(10, false);
        assert_eq!(false, vec.get_bit(10));
    }

    #[test]
    fn test_set_block() {
        let mut vec = BitVec::new();
        vec.set_block(4, !0);
        assert_eq!(false, vec.get_bit(3));
        assert_eq!(true, vec.get_bit(4));
        assert_eq!(true, vec.get_bit(67));
        assert_eq!(false, vec.get_bit(68));
    }

    #[test]
    fn test_get_block() {
        let mut vec = BitVec::new();
        vec.set_block(0, !0);
        assert_eq!(!0, vec.get_block(0));
        vec.set_block(67, !0);
        assert_eq!(!0, vec.get_block(67));
        vec.set_block(256, !0);
        assert_eq!(!0, vec.get_block(256));
        assert_eq!(!0u64 << 2, vec.get_block(258));
    }

    #[test]
    fn test_clear() {
        let mut vec = BitVec::new();
        vec.set_block(0, !0);
        vec.clear();
        assert_eq!(0, vec.get_block(0));
    }

    #[test]
    fn test_append() {
        let mut vec = AppendOnlyBitVec::new();
        vec.append(1, 0);
        assert_eq!(false, vec.get_bit(0));
        vec.append(0, 1);
        assert_eq!(false, vec.get_bit(0));
        vec.append(1, 1);
        assert_eq!(false, vec.get_bit(0));
        assert_eq!(true, vec.get_bit(1));
        vec.append(1, 0); // buffer
        vec.append(64, !0);
        assert_eq!(!0 >> 1, vec.get_block(2));
        assert_eq!(!0, vec.get_block(3));
        vec.append(3, 3);
        assert_eq!(false, vec.get_bit(68));
        assert_eq!(true, vec.get_bit(69));
        assert_eq!(true, vec.get_bit(70));
        assert_eq!(false, vec.get_bit(71));
    }
}
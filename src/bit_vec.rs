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

    pub fn get_bit(&self, index: usize) -> u64 {
        if self.out_of_bounds(index) {
            return 0;
        }

        let block = self.block(index);
        let offset = offset_i(index);
        let mask = 1 << offset;
        (block & mask) >> offset
    }

    pub fn set_bit(&mut self, index: usize, value: bool) {
        if self.out_of_bounds(index) {
            self.data.resize(blocks(index + 1), 0);
        }

        let mut block = self.block_mut(index);
        let offset = offset_i(index);
        if value {
            let mask = 1 << offset;
            *block |= mask
        } else {
            let mask = !(1 << offset);
            *block &= mask
        }
    }

    pub fn set_block(&mut self, index: usize, block: u64) {
        if self.out_of_bounds(index) {
            if index % 64 == 0 {
                // offset is block-aligned, no need to add
                // extra block for new block
                self.data.resize(blocks(index + 1), 0);
            } else {
                // offset is not block-aligned, new block will extend
                // past number of blocks needed to cover index + 1 bits
                self.data.resize(blocks(index + 1) + 1, 0);
            }
        }

        //  Algorithm example:
        //
        //  Let index = 67
        //
        //  Blocks after resize:
        //
        //  |--------||--------||--------|
        //  |    0   ||    1   ||   2    |
        //  |--------||--------||--------|
        //                 ^
        //                 |
        //             cur_block (67 % 64 == 3, ergo 67 / 64 + 1 = 1)
        //
        //  Block 1:
        //
        //  |----------------------------------------------------------------|
        //  |0000000000000000000000000000000000000000000000000000000000000000|
        //  |----------------------------------------------------------------|
        //      ^
        //      |
        //    cur_offset (canonical index: 3, actual bit index: 60)
        //    63 - 67 % 64 = 60. We do 63 - x so that bit indexes start
        //    at bit 63 and work to bit 0 so that blocks are contiguous
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
        self.set_cur_block(index, block);
        if index % 64 != 0 {
            self.set_next_block(index, block);
        }
    }

    fn set_cur_block(&mut self, index: usize, block: u64) {
        let mut cur_block = self.block_mut(index);
        let offset = offset_i(index);
        let mask = !0 << (offset + 1);
        let data = block >> (64 - (offset + 1));
        println!("{:b}", mask >> 4);
        *cur_block = (*cur_block & mask) | data;
    }

    fn set_next_block(&mut self, index: usize, block: u64) {
        let mut cur_block = &mut self.data[block_i(index) + 1];
        let offset = offset_i(index);
        let mask = !0 >> (64 - (offset + 1));
        let data = block << (offset + 1);
        *cur_block = (*cur_block & mask) | data;
    }

    fn block(&self, index: usize) -> &u64 {
        &self.data[block_i(index)]
    }

    fn block_mut(&mut self, index: usize) -> &mut u64 {
        &mut self.data[block_i(index)]
    }

    fn out_of_bounds(&self, index: usize) -> bool {
        blocks(index + 1) > self.data.len()
    }
}

/// Returns the 0-based index of the block given the index
fn block_i(index: usize) -> usize {
    index / 64
}

/// Returns the 0-based block offset for index
fn offset_i(index: usize) -> usize {
    63 - index % 64
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
    use super::BitVec;

    #[test]
    fn test_get_set() {
        let mut vec = BitVec::new();
        vec.set_bit(0, true);
        vec.set_bit(5, true);
        vec.set_bit(10, true);
        assert_eq!(1, vec.get_bit(0));
        assert_eq!(1, vec.get_bit(5));
        assert_eq!(1, vec.get_bit(10));
        assert_eq!(0, vec.get_bit(32));
        vec.set_bit(10, false);
        assert_eq!(0, vec.get_bit(10));
    }

    #[test]
    fn test_set_block() {
        let mut vec = BitVec::new();
        vec.set_block(3, !0);
        println!("{:b}", vec.data[0]);
        assert_eq!(1, 2);
    }
}
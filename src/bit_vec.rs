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
        let offset = offset(index);
        let mask = 1 << offset;
        (block & mask) >> offset
    }

    pub fn set_bit(&mut self, index: usize, value: bool) {
        if self.out_of_bounds(index) {
            self.data.resize(blocks(index + 1), 0);
        }

        let mut block = self.block_mut(index);
        let offset = offset(index);
        if value {
            let mask = 1 << offset;
            *block |= mask
        } else {
            let mask = !(1 << offset);
            *block &= mask
        }
    }

    fn block(&self, index: usize) -> &u64 {
        &self.data[blocks(index + 1) - 1]
    }

    fn block_mut(&mut self, index: usize) -> &mut u64 {
        &mut self.data[blocks(index + 1) - 1]
    }

    fn out_of_bounds(&self, index: usize) -> bool {
        blocks(index + 1) > self.data.len()
    }
}

/// Returns the number of 32 bit blocks it takes to contain
/// nbits
fn blocks(nbits: usize) -> usize {
    match nbits % 64 {
        0 => nbits >> 6,
        _ => (nbits >> 6) + 1,
    }
}

/// Returns the block offset for nbits
fn offset(nbits: usize) -> usize {
    nbits % 64
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
}
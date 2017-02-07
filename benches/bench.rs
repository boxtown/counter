#![feature(test)]

extern crate test;
extern crate counter;

#[cfg(test)]
mod tests {
    use test::Bencher;
    use counter::bit_vec::BitVec;

    #[bench]
    fn bench_len(bench: &mut Bencher) {
        let mut v = BitVec::new();
        v.set_bit(128, true);
        bench.iter(|| v.len())
    }

    #[bench]
    fn bench_set_bit(bench: &mut Bencher) {
        let mut i = 0;
        let mut v = BitVec::new();
        bench.iter(|| {
            v.set_bit(i, true);
            i += 1;
        })
    }

    #[bench]
    fn bench_get_bit(bench: &mut Bencher) {
        let mut v = BitVec::new();
        v.set_bit(512, true);
        bench.iter(|| v.get_bit(100))
    }

    #[bench]
    fn bench_set_block(bench: &mut Bencher) {
        let mut i = 0;
        let mut v = BitVec::new();
        bench.iter(|| {
            v.set_block(i, 1);
            i += 1;
        })
    }

    #[bench]
    fn bench_get_block(bench: &mut Bencher) {
        let mut v = BitVec::new();
        v.set_block(105, !0);
        bench.iter(|| v.get_block(102))
    }
}
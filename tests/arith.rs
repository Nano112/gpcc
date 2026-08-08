// SPDX-License-Identifier: MIT
use gpcc::arith::{ArithDecoder, Init};

/// Deterministic pseudo-random bytes; avoids a `rand` dependency.
fn lcg(n: usize, seed: u32) -> Vec<u8> {
    let mut s = seed;
    (0..n)
        .map(|_| {
            s = s.wrapping_mul(1664525).wrapping_add(1013904223);
            (s >> 24) as u8
        })
        .collect()
}

/// A correct arithmetic decoder driven at p=0.5 with no adaptation degenerates
/// exactly into a raw bit reader: ~50% ones, short runs, and about one input
/// byte per eight output bits.
#[test]
fn bypass_degenerates_to_a_bit_reader() {
    let data = lcg(4096, 0x1234_5678);
    let mut d = ArithDecoder::new(&data);
    let n = 8000;
    let (mut ones, mut run, mut max_run, mut prev) = (0usize, 1usize, 1usize, 2u8);
    for _ in 0..n {
        let bit = d.decode_bypass();
        ones += usize::from(bit);
        if bit == prev {
            run += 1;
            max_run = max_run.max(run);
        } else {
            run = 1;
        }
        prev = bit;
    }
    let pct = 100.0 * ones as f64 / n as f64;
    assert!((40.0..60.0).contains(&pct), "ones = {pct:.1}%, expected ~50%");
    assert!(max_run < 40, "longest run {max_run}: decoder looks desynchronised");
    let used = d.bytes_consumed();
    assert!((900..1150).contains(&used), "consumed {used} bytes for 8000 bits");
}

/// Both init variants are valid entry points and must behave identically in
/// bypass mode, differing only in how much of the stream is preloaded.
#[test]
fn both_inits_are_healthy_in_bypass() {
    let data = lcg(4096, 0xC0FF_EE01);
    for init in [Init::Schro, Init::Preload4] {
        let mut d = ArithDecoder::with_init(&data, init);
        let ones: usize = (0..4000).map(|_| usize::from(d.decode_bypass())).sum();
        let pct = 100.0 * ones as f64 / 4000.0;
        assert!((40.0..60.0).contains(&pct), "{init:?}: ones = {pct:.1}%");
    }
}

/// An adaptive context must actually move away from its 0x8000 start.
#[test]
fn adaptive_context_updates() {
    let data = vec![0u8; 512];
    let mut d = ArithDecoder::new(&data);
    let mut ctx: u16 = 0x8000;
    for _ in 0..64 {
        d.decode_bit(&mut ctx);
    }
    assert_ne!(ctx, 0x8000, "context never adapted");
}

/// Reading past the end must not panic; schro feeds 0xff forever.
#[test]
fn runs_off_the_end_without_panicking() {
    let data = [0xAAu8; 4];
    let mut d = ArithDecoder::new(&data);
    for _ in 0..1000 {
        d.decode_bypass();
    }
    assert!(d.exhausted());
}

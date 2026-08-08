// SPDX-License-Identifier: BSD-3-Clause
//! Integration tests for the DualLutCoder symbol decoder.

use gpcc::arith::ArithDecoder;
use gpcc::dual_lut::DualLutCoder;

/// Deterministic pseudo-random buffer (LCG), so tests are reproducible.
fn lcg_buffer(len: usize, seed: u32) -> Vec<u8> {
    let mut state = seed;
    (0..len)
        .map(|_| {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            (state >> 24) as u8
        })
        .collect()
}

fn decode_n<const N: usize, const LIMITED: bool>(data: &[u8], count: usize) -> Vec<u8> {
    let mut dec = ArithDecoder::new(data);
    let mut coder = DualLutCoder::<N, LIMITED>::new();
    (0..count).map(|_| coder.decode(&mut dec)).collect()
}

#[test]
fn initial_lut_is_identity() {
    let coder = DualLutCoder::<256>::new();
    let expected: Vec<u8> = (0..32).collect();
    assert_eq!(coder.lut_symbols(), expected.as_slice());
}

#[test]
fn symbols_are_in_alphabet_small_n() {
    // For N < 256 every decoded symbol must still land inside the alphabet,
    // even when the direct-coding path fires on arbitrary input.
    let data = lcg_buffer(8192, 0xdead_beef);
    for symbol in decode_n::<16, true>(&data, 1000) {
        assert!(symbol < 16, "symbol {symbol} outside 16-ary alphabet");
    }
    for symbol in decode_n::<7, true>(&data, 1000) {
        assert!(symbol < 7, "symbol {symbol} outside 7-ary alphabet");
    }
}

#[test]
fn symbols_are_in_alphabet_full_context_mode() {
    let data = lcg_buffer(8192, 0x1234_5678);
    for symbol in decode_n::<16, false>(&data, 1000) {
        assert!(symbol < 16, "symbol {symbol} outside 16-ary alphabet");
    }
    // N = 256: every u8 is in-alphabet by construction; just ensure it runs.
    let symbols = decode_n::<256, false>(&data, 1000);
    assert_eq!(symbols.len(), 1000);
}

#[test]
fn coder_adapts_lut_to_observed_symbols() {
    // Feeding varied symbols must eventually reorder the frequency-sorted LUT
    // away from its initial identity mapping.
    let data = lcg_buffer(16384, 0xcafe_f00d);
    let mut dec = ArithDecoder::new(&data);
    let mut coder = DualLutCoder::<256>::new();

    let before: Vec<u8> = coder.lut_symbols().to_vec();
    for _ in 0..2000 {
        coder.decode(&mut dec);
    }
    let after: Vec<u8> = coder.lut_symbols().to_vec();

    assert_eq!(before.len(), 32);
    assert_eq!(after.len(), 32);
    assert_ne!(before, after, "LUT never adapted after 2000 symbols");
}

#[test]
fn determinism_same_input_same_output() {
    let data = lcg_buffer(4096, 42);
    let first = decode_n::<256, true>(&data, 800);
    let second = decode_n::<256, true>(&data, 800);
    assert_eq!(first, second);

    let first = decode_n::<256, false>(&data, 800);
    let second = decode_n::<256, false>(&data, 800);
    assert_eq!(first, second);
}

#[test]
fn no_panic_on_adversarial_input() {
    let buffers: Vec<Vec<u8>> = vec![
        vec![],
        vec![0x00],
        vec![0xff],
        vec![0x00; 4096],
        vec![0xff; 4096],
        (0..4096).map(|i| if i % 2 == 0 { 0x00 } else { 0xff }).collect(),
        lcg_buffer(4096, 0x0bad_5eed),
    ];

    for data in &buffers {
        // Both context modes, big and small alphabets. Nothing here may panic,
        // and every symbol must stay in-alphabet.
        let symbols = decode_n::<256, true>(data, 2000);
        assert_eq!(symbols.len(), 2000);
        let symbols = decode_n::<256, false>(data, 2000);
        assert_eq!(symbols.len(), 2000);
        for symbol in decode_n::<16, true>(data, 2000) {
            assert!(symbol < 16);
        }
        for symbol in decode_n::<3, false>(data, 2000) {
            assert!(symbol < 3);
        }
    }
}

#[test]
fn decoding_past_end_of_buffer_does_not_panic() {
    // Tiny buffer, many symbols: the arithmetic decoder runs off the end
    // (yielding 0xff fill bytes) and the symbol coder must stay well-behaved.
    let data = [0x5a, 0xa5];
    let symbols = decode_n::<256, true>(&data, 5000);
    assert_eq!(symbols.len(), 5000);
}

#[test]
fn reset_lut_does_not_disturb_decoding() {
    let data = lcg_buffer(4096, 7);
    let mut dec = ArithDecoder::new(&data);
    let mut coder = DualLutCoder::<256>::new();
    for _ in 0..100 {
        coder.decode(&mut dec);
    }
    coder.reset_lut();
    for _ in 0..500 {
        let _ = coder.decode(&mut dec);
    }
}

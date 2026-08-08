// SPDX-License-Identifier: MIT
use gpcc::arith::ArithDecoder;
use gpcc::golomb::{decode_exp_golomb, decode_signed_exp_golomb};
use gpcc::Error;

/// Deterministic pseudo-random bytes. Keeps the test suite self-contained — no
/// external data files, no dependency on any particular application format.
fn lcg(n: usize, seed: u32) -> Vec<u8> {
    let mut s = seed;
    (0..n)
        .map(|_| {
            s = s.wrapping_mul(1664525).wrapping_add(1013904223);
            (s >> 24) as u8
        })
        .collect()
}

/// Golden vector locking the whole entropy stack: arithmetic decoder,
/// probability updates and exp-Golomb binarisation together. Any drift in any of
/// them changes these numbers.
const GOLDEN: [u32; 40] = [
    0, 1, 0, 6, 2, 41, 6, 1, 6, 0, 0, 0, 7, 2, 6, 0, 0, 1, 47, 0, 1, 0, 1, 2, 0, 6, 0, 0, 1, 2, 6,
    0, 0, 36, 16, 0, 95, 0, 33, 44,
];

#[test]
fn reproduces_the_reference_value_sequence() {
    let data = lcg(4096, 0x5EED_1234);
    let mut d = ArithDecoder::new(&data);
    let mut ctx: u16 = 0x8000;
    let got: Vec<u32> = (0..GOLDEN.len())
        .map(|_| decode_exp_golomb(&mut d, 0, &mut ctx).expect("decode"))
        .collect();
    assert_eq!(got, GOLDEN, "entropy stack drifted from the reference");
}

/// The same input must terminate at the same point. Random bytes are not a
/// valid arithmetic code, so the decoder eventually rejects one — but it does so
/// reproducibly, which is itself a fingerprint.
#[test]
fn terminates_where_the_reference_does() {
    let data = lcg(4096, 0x5EED_1234);
    let mut d = ArithDecoder::new(&data);
    let mut ctx: u16 = 0x8000;
    let mut n = 0usize;
    while decode_exp_golomb(&mut d, 0, &mut ctx).is_ok() {
        n += 1;
        if n > 100_000 {
            break;
        }
    }
    assert_eq!(n, 51, "symbol count before desync diverged from the reference");
}

/// An unbounded prefix means lost sync: it must be a clean error, never a hang
/// or an overflow. All-0xff drives the decoder to emit 1 forever.
#[test]
fn unbounded_prefix_is_an_error_not_a_hang() {
    let data = vec![0xffu8; 64];
    let mut d = ArithDecoder::new(&data);
    let mut ctx: u16 = 0x8000;
    match decode_exp_golomb(&mut d, 0, &mut ctx) {
        Err(Error::Desync(m)) => assert!(m.contains("prefix"), "unexpected message: {m}"),
        other => panic!("expected Desync, got {other:?}"),
    }
}

/// All-zero input drives the decoder to emit 0 forever, which is a valid,
/// terminating exp-Golomb symbol. This must not be mistaken for desync.
#[test]
fn all_zero_input_yields_zeros_not_an_error() {
    let data = vec![0u8; 64];
    let mut d = ArithDecoder::new(&data);
    let mut ctx: u16 = 0x8000;
    for _ in 0..64 {
        assert_eq!(decode_exp_golomb(&mut d, 0, &mut ctx).expect("decode"), 0);
    }
}

/// The signed form must produce zero, negative and positive values, and must
/// skip the sign bin when the magnitude is zero.
#[test]
fn signed_form_produces_both_signs() {
    let data = lcg(8192, 0xC0FF_EE01);
    let mut d = ArithDecoder::new(&data);
    let (mut m, mut s) = (0x8000u16, 0x8000u16);
    let (mut neg, mut pos, mut zero) = (false, false, false);
    for _ in 0..400 {
        match decode_signed_exp_golomb(&mut d, 0, &mut m, &mut s) {
            Ok(0) => zero = true,
            Ok(v) if v < 0 => neg = true,
            Ok(_) => pos = true,
            Err(_) => break,
        }
    }
    assert!(zero && neg && pos, "expected zero, negative and positive values");
}

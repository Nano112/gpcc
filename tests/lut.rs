// SPDX-License-Identifier: MIT
use gpcc::PROB_UPDATE_LUT;

/// The table is 256 interleaved pairs: even index applies when the decoded bit
/// is 0, odd when it is 1. Values checked against `research/ac_init_lut.bin`.
#[test]
fn lut_matches_reference_table() {
    assert_eq!(PROB_UPDATE_LUT.len(), 512);
    assert_eq!(
        &PROB_UPDATE_LUT[..12],
        &[255, 0, 376, -2, 471, -5, 553, -8, 625, -11, 690, -15]
    );
    assert!(PROB_UPDATE_LUT[0] > 0 && PROB_UPDATE_LUT[1] == 0);
    assert_eq!(PROB_UPDATE_LUT[510], 0);
    assert_eq!(PROB_UPDATE_LUT[511], -255);
}

/// Even entries rise then fall, odd entries mirror them — the shape of a
/// probability state-transition table. A truncated or misaligned extraction
/// would break this.
#[test]
fn lut_is_an_interleaved_mirror() {
    for i in 0..256 {
        assert_eq!(
            PROB_UPDATE_LUT[2 * i],
            -PROB_UPDATE_LUT[2 * (255 - i) + 1],
            "pair {i} is not the mirror of pair {}",
            255 - i
        );
    }
}

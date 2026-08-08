// SPDX-License-Identifier: MIT
// Exp-Golomb binarisation is a standard construction predating this codebase.
// TMC13 `tmc3/entropyutils.h` was consulted as a reference implementation.
//! Exp-Golomb binarisation over the arithmetic decoder.
//!
//! Direct port of TMC13's `EntropyDecoderWrapper::decodeExpGolomb`:
//!
//! ```text
//! do { l = decode(ctxPrefix);
//!      if (l == 1) { symbol += (1 << k); k++; }
//! } while (l != 0);
//! while (k--) if (decode() == 1) binary_symbol |= (1 << k);
//! return symbol + binary_symbol;
//! ```
//!
//! Note the suffix bits are bypass-coded in the single-context form, and
//! context-coded in the multi-context form.

use crate::arith::ArithDecoder;
use crate::{Error, Result};

/// A prefix longer than this means lost sync: a real residual never needs it,
/// and without the bound the shift below overflows. C silently wraps a 32-bit
/// int here; we fail loudly instead.
pub const MAX_EG_K: u32 = 48;

/// Adaptive prefix, bypass-coded suffix.
pub fn decode_exp_golomb(d: &mut ArithDecoder, k: u32, ctx: &mut u16) -> Result<u32> {
    let mut k = k;
    let mut symbol: u64 = 0;
    while d.decode_bit(ctx) == 1 {
        symbol = symbol.saturating_add(1u64 << k.min(63));
        k += 1;
        if k > MAX_EG_K {
            return Err(Error::Desync(format!(
                "exp-Golomb prefix exceeded k={MAX_EG_K}"
            )));
        }
    }
    let mut suffix: u64 = 0;
    while k > 0 {
        k -= 1;
        if d.decode_bypass() == 1 {
            suffix |= 1u64 << k;
        }
    }
    u32::try_from(symbol.saturating_add(suffix))
        .map_err(|_| Error::Desync("exp-Golomb value exceeds u32".into()))
}

/// Multi-context form: prefix bins indexed by `k - k0`, suffix bins by `k`,
/// both clamped to the supplied slice lengths.
pub fn decode_exp_golomb_ctx(
    d: &mut ArithDecoder,
    k: u32,
    prefix: &mut [u16],
    suffix: &mut [u16],
) -> Result<u32> {
    if prefix.is_empty() || suffix.is_empty() {
        return Err(Error::InvalidParam {
            param: "context banks",
            detail: "must be non-empty".into(),
        });
    }
    let k0 = k;
    let mut k = k;
    let mut symbol: u64 = 0;
    loop {
        let i = ((k - k0) as usize).min(prefix.len() - 1);
        if d.decode_bit(&mut prefix[i]) == 0 {
            break;
        }
        symbol = symbol.saturating_add(1u64 << k.min(63));
        k += 1;
        if k > MAX_EG_K {
            return Err(Error::Desync(format!(
                "exp-Golomb prefix exceeded k={MAX_EG_K}"
            )));
        }
    }
    let mut out: u64 = 0;
    while k > 0 {
        k -= 1;
        let i = (k as usize).min(suffix.len() - 1);
        out |= u64::from(d.decode_bit(&mut suffix[i])) << k;
    }
    u32::try_from(symbol.saturating_add(out))
        .map_err(|_| Error::Desync("exp-Golomb value exceeds u32".into()))
}

/// Magnitude then sign, each with its own adaptive context.
///
/// The sign bin is skipped when the magnitude is zero, so a zero costs no sign
/// bit. This is the signed counterpart to [`decode_exp_golomb`].
pub fn decode_signed_exp_golomb(
    d: &mut ArithDecoder,
    k: u32,
    mag_ctx: &mut u16,
    sign_ctx: &mut u16,
) -> Result<i32> {
    let mag = decode_exp_golomb(d, k, mag_ctx)?;
    if mag == 0 {
        return Ok(0);
    }
    let m = i64::from(mag);
    let signed = if d.decode_bit(sign_ctx) == 1 { -m } else { m };
    i32::try_from(signed).map_err(|_| Error::Desync("signed exp-Golomb exceeds i32".into()))
}

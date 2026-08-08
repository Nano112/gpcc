// SPDX-License-Identifier: MIT
// Derived from Schroedinger `schroarith.h` (quad-licensed MPL-1.1/LGPL/GPL/MIT;
// MIT elected here). See THIRD-PARTY-NOTICES.md.
//! Schroedinger/Dirac binary arithmetic decoder.
//!
//! Ported from `_schro_arith_decode_bit` in TMC13's
//! `dependencies/schroedinger/schroarith.h`:
//!
//! ```text
//! while (range <= 0x40000000) {
//!   if (!--cntr) { code |= read() << 8; cntr = 8; }
//!   range <<= 1; code <<= 1;
//! }
//! range_x_prob = ((range >> 16) * prob) & 0xFFFF0000;
//! lut_index    = (prob >> 7) & ~1;
//! value        = (code >= range_x_prob);
//! prob        += lut[lut_index | value];
//! if (value) { code -= range_x_prob; range -= range_x_prob; }
//! else       { range = range_x_prob; }
//! ```

use crate::PROB_UPDATE_LUT;

/// How the decoder primes its code register.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Init {
    /// Two preloaded bytes, `cntr = 1` — stock `schro_arith_decode_init`.
    /// This is what real streams use; see [`ArithDecoder::new`].
    Schro,
    /// Four preloaded bytes, `cntr = 3`.
    ///
    /// A variant found in deployed encoders, which prime the code register with
    /// four bytes instead of two. Both variants decode the same bit sequence in
    /// bypass mode; they differ only in how much of the stream is buffered up
    /// front, so a decoder must be told which one produced the stream.
    Preload4,
}

pub struct ArithDecoder<'a> {
    data: &'a [u8],
    pos: usize,
    code: u32,
    range: u32,
    cntr: i32,
}

impl<'a> ArithDecoder<'a> {
    /// Decoder using [`Init::Schro`], the standard two-byte priming.
    ///
    /// Verified against real streams: with this init, the first coded value of
    /// all 108 blocks in the reference corpus reproduces exactly. The
    /// four-byte [`Init::Preload4`] variant does not.
    pub fn new(data: &'a [u8]) -> Self {
        Self::with_init(data, Init::Schro)
    }

    pub fn with_init(data: &'a [u8], init: Init) -> Self {
        let n = match init {
            Init::Schro => 2,
            Init::Preload4 => 4,
        };
        let mut code: u32 = 0;
        for i in 0..n {
            let b = u32::from(*data.get(i).unwrap_or(&0xff));
            code |= b << (24 - 8 * i);
        }
        Self {
            data,
            pos: n,
            code,
            range: 0xffff_0000,
            cntr: if matches!(init, Init::Schro) { 1 } else { 3 },
        }
    }

    /// Bytes drawn from the stream so far, including the preloaded ones.
    pub fn bytes_consumed(&self) -> usize {
        self.pos
    }

    /// True once reads have run past the end of the buffer (which yield 0xff,
    /// matching schro). Useful as a sanity signal, not an error on its own.
    pub fn exhausted(&self) -> bool {
        self.pos > self.data.len()
    }

    #[inline]
    fn read_byte(&mut self) -> u32 {
        let b = u32::from(*self.data.get(self.pos).unwrap_or(&0xff));
        self.pos = self.pos.saturating_add(1);
        b
    }

    /// Decode one bit against an adaptive context (a u16 probability state).
    pub fn decode_bit(&mut self, ctx: &mut u16) -> u8 {
        let mut range = self.range;
        let mut code = self.code;

        while range <= 0x4000_0000 {
            self.cntr -= 1;
            if self.cntr == 0 {
                let b = self.read_byte();
                code |= b << 8;
                self.cntr = 8;
            }
            range = range.wrapping_shl(1);
            code = code.wrapping_shl(1);
        }

        let prob = u32::from(*ctx);
        let range_x_prob = (range >> 16).wrapping_mul(prob) & 0xffff_0000;
        let value = u8::from(code >= range_x_prob);

        let idx = (((prob >> 7) & !1u32) | u32::from(value)) as usize;
        let delta = i32::from(PROB_UPDATE_LUT[idx]);
        *ctx = ((prob as i32).wrapping_add(delta) as u32 & 0xffff) as u16;

        if value == 1 {
            code = code.wrapping_sub(range_x_prob);
            range = range.wrapping_sub(range_x_prob);
        } else {
            range = range_x_prob;
        }

        self.range = range;
        self.code = code;
        value
    }

    /// Equiprobable bit: a fresh p=0.5 context each call, so nothing adapts.
    /// Mirrors TMC13's `ArithmeticDecoder::decode()`.
    pub fn decode_bypass(&mut self) -> u8 {
        let mut p: u16 = 0x8000;
        self.decode_bit(&mut p)
    }
}

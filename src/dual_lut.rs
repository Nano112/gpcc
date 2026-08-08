// SPDX-License-Identifier: BSD-3-Clause
// Derived from TMC13 `tmc3/DualLutCoder.{h,cpp}`, Copyright (c) 2017-2018 ISO/IEC.
// This file, unlike the rest of the crate, carries BSD-3-Clause obligations.
// See THIRD-PARTY-NOTICES.md.
//! DualLutCoder: the two-tier adaptive symbol coder from MPEG G-PCC
//! (ISO/IEC 23090-9).
//!
//! Ported from the TMC13 reference implementation
//! (`tmc3/DualLutCoder.{h,cpp}`, MPEGGroup/mpeg-pcc-tmc13, BSD-licensed,
//! Copyright (c) 2017-2018, ISO/IEC).
//!
//! A symbol from an `N`-ary alphabet (N <= 256) is coded with one of three
//! methods, tried in order:
//!
//! 1. **Frequency-sorted LUT** (`FrequencySortingLut`, 32 entries): the 32
//!    most frequent symbols, resorted periodically from a per-symbol histogram
//!    with an exponentially backed-off update period. A hit codes a 5-bit LUT
//!    index; small indices (frequent symbols) get adaptive contexts.
//! 2. **LRU cache** (`FrequentSymbolCache`, 16 entries): recently seen
//!    symbols that missed the LUT. A hit codes a 4-bit index in bypass mode.
//! 3. **Direct**: the raw 8-bit symbol, each bit through one shared adaptive
//!    context.
//!
//! Two flavours exist in TMC13, selected by its `_limitedContextMode` template
//! parameter, mirrored here by the `LIMITED` const generic (default `true`):
//!
//! * `LIMITED = true`: 5 contexts for the LUT index, coded as a unary-style
//!   prefix over the index bits (suffix bits are bypass); histogram counts and
//!   update period both cap at 1024.
//! * `LIMITED = false`: 31 contexts forming a full binary context tree over
//!   the 5 index bits; update period caps at `0x3333_3333` and histogram
//!   counts at `1 << 24`.
//!
//! Only the decoder is implemented, on top of [`ArithDecoder`].

use crate::arith::ArithDecoder;

/// log2 of the frequency-sorted LUT size.
const LUT_LOG2: usize = 5;
/// Frequency-sorted LUT size (TMC13 `kLutSize`).
const LUT_SIZE: usize = 1 << LUT_LOG2;
/// log2 of the LRU cache size (TMC13 `kLog2CacheSize`).
const CACHE_LOG2: usize = 4;
/// LRU cache size (TMC13 `kCacheSize`).
const CACHE_SIZE: usize = 1 << CACHE_LOG2;
/// Sentinel for "symbol not present" (TMC13 `kUndefinedIndex`).
const UNDEFINED: i16 = -1;
/// Initial LUT update period (TMC13 `kInitialUpdatePeriod`).
const INITIAL_UPDATE_PERIOD: u32 = 16;
/// Fresh adaptive context (p = 0.5) for the arithmetic decoder.
const CTX_INIT: u16 = 0x8000;

/// A forward and inverse symbol <-> frequency-sorted-index lookup table.
///
/// Maintains mappings for the [`LUT_SIZE`] most frequent symbols of an
/// `N`-symbol alphabet. Mappings are recomputed from per-symbol occurrence
/// counts on a periodic basis following symbol insertion, with an exponential
/// back-off up to a maximum update period.
///
/// Port of TMC13 `FrequencySortingLut<_lutSize, _alphabetSize>`.
struct FrequencySortingLut<const N: usize> {
    /// Per-symbol occurrence counts.
    histogram: [u32; N],
    /// Symbol -> LUT index, [`UNDEFINED`] when absent.
    to_index: [i16; N],
    /// LUT index -> symbol.
    to_symbol: [u8; LUT_SIZE],
    /// Histogram ceiling; on reaching it all counts are halved.
    max_histogram_count: u32,
    /// Cap for the exponentially growing update period.
    max_update_period: u32,
    update_period: u32,
    symbols_until_update: u32,
    /// When set, histogram counts are cleared at the next `update()`.
    reset: bool,
}

impl<const N: usize> FrequencySortingLut<N> {
    /// Effective LUT length: the LUT never exceeds the alphabet.
    const LUT_LEN: usize = if N < LUT_SIZE { N } else { LUT_SIZE };

    fn new(max_update_period: u32, max_histogram_count: u32) -> Self {
        let mut lut = Self {
            histogram: [0; N],
            to_index: [UNDEFINED; N],
            to_symbol: [0; LUT_SIZE],
            max_histogram_count,
            max_update_period,
            update_period: INITIAL_UPDATE_PERIOD,
            symbols_until_update: INITIAL_UPDATE_PERIOD,
            reset: false,
        };
        lut.init();
        lut
    }

    /// Initialise with the identity mapping (TMC13 `init` with no init table:
    /// LUT slot `k` holds symbol `k`, with a histogram count of 1).
    fn init(&mut self) {
        for k in 0..N {
            self.histogram[k] = 0;
            self.to_index[k] = UNDEFINED;
        }
        for k in 0..Self::LUT_LEN {
            self.to_symbol[k] = k as u8;
            self.to_index[k] = k as i16;
            self.histogram[k] = 1;
        }
    }

    /// Schedule a histogram reset at the next periodic `update()`.
    fn reset(&mut self) {
        self.reset = true;
    }

    /// Recompute the LUT from the histogram (TMC13 `update`).
    fn update(&mut self) {
        // Exponential back-off: period *= 1.25, capped. TMC13 notes this
        // expression limits the maximum period to 0x33333333.
        self.update_period = self
            .update_period
            .saturating_mul(5)
            .wrapping_shr(2)
            .min(self.max_update_period);
        self.symbols_until_update = self.update_period;

        // Sort symbols by descending occurrence count. TMC13 packs
        // `((~histogram[s]) << 8) + s` into a u32 so that an ascending sort
        // yields descending counts with the symbol value breaking ties
        // (guaranteeing stability). TMC13 uses nth_element + sort of the first
        // LUT_LEN entries; a full sort selects the identical set and order.
        let mut tmp = [0u32; N];
        for (symbol, slot) in tmp.iter_mut().enumerate() {
            *slot = (!self.histogram[symbol])
                .wrapping_shl(8)
                .wrapping_add(symbol as u32 & 0xff);
        }
        tmp.sort_unstable();

        // Remove existing mappings.
        for k in 0..Self::LUT_LEN {
            let old = usize::from(self.to_symbol[k]);
            if let Some(idx) = self.to_index.get_mut(old) {
                *idx = UNDEFINED;
            }
        }

        // Re-populate both directions.
        for (k, key) in tmp.iter().enumerate().take(Self::LUT_LEN) {
            let symbol = (key & 0xff) as u8;
            self.to_symbol[k] = symbol;
            if let Some(idx) = self.to_index.get_mut(usize::from(symbol)) {
                *idx = k as i16;
            }
        }

        // Deferred histogram reset (TMC13 `_reset`).
        if self.reset {
            self.reset = false;
            for count in &mut self.histogram {
                *count = 0;
            }
            for k in 0..Self::LUT_LEN {
                let symbol = usize::from(self.to_symbol[k]);
                if let Some(count) = self.histogram.get_mut(symbol) {
                    *count = 1;
                }
            }
        }
    }

    /// Update the histogram for `symbol` and recompute the LUT if the update
    /// period has elapsed (TMC13 `pushSymbol`). Out-of-alphabet symbols are
    /// ignored (unreachable when N = 256).
    fn push_symbol(&mut self, symbol: usize) {
        let Some(count) = self.histogram.get_mut(symbol) else {
            return;
        };
        *count = count.saturating_add(1);
        if *count >= self.max_histogram_count {
            for count in &mut self.histogram {
                *count >>= 1;
            }
        }

        self.symbols_until_update = self.symbols_until_update.saturating_sub(1);
        if self.symbols_until_update == 0 {
            self.update();
        }
    }

    /// The symbol stored at LUT `index`; 0 for an out-of-range index
    /// (unreachable when N >= [`LUT_SIZE`]).
    fn get_symbol(&self, index: usize) -> u8 {
        self.to_symbol.get(index).copied().unwrap_or(0)
    }

    /// The LUT slots in order, most frequent first.
    fn symbols(&self) -> &[u8] {
        &self.to_symbol[..Self::LUT_LEN]
    }
}

/// A forward and inverse cache of recently used symbols with a
/// least-recently-used-style (round-robin slot) eviction policy.
///
/// Port of TMC13 `FrequentSymbolCache<_cacheSize, _alphabetSize>`.
struct FrequentSymbolCache<const N: usize> {
    /// Symbol -> cache index, [`UNDEFINED`] when absent.
    to_index: [i16; N],
    /// Cache index -> symbol.
    to_symbol: [u8; CACHE_SIZE],
    /// Round-robin eviction cursor (TMC13 `_last`).
    last: u32,
}

impl<const N: usize> FrequentSymbolCache<N> {
    /// Effective cache length: the cache never exceeds the alphabet.
    const CACHE_LEN: usize = if N < CACHE_SIZE { N } else { CACHE_SIZE };

    fn new() -> Self {
        let mut cache = Self {
            to_index: [UNDEFINED; N],
            to_symbol: [0; CACHE_SIZE],
            last: 0,
        };
        for k in 0..Self::CACHE_LEN {
            cache.to_symbol[k] = k as u8;
            cache.to_index[k] = k as i16;
        }
        cache
    }

    /// Insert `symbol`, evicting the symbol in the round-robin victim slot
    /// (TMC13 `pushSymbol`). Out-of-alphabet symbols are ignored (unreachable
    /// when N = 256).
    fn push_symbol(&mut self, symbol: usize) {
        if symbol >= N || Self::CACHE_LEN == 0 {
            return;
        }
        let index = self.to_index[symbol];
        let index0 = (self.last as usize) % Self::CACHE_LEN;
        self.last = self.last.wrapping_add(1);
        let symbol0 = usize::from(self.to_symbol[index0]);
        if symbol0 >= N {
            // Cannot occur: every cached symbol was inserted via this method.
            return;
        }

        self.to_index.swap(symbol, symbol0);
        if index == UNDEFINED {
            self.to_symbol[index0] = symbol as u8;
        } else {
            let index = index as usize;
            if index < CACHE_SIZE {
                self.to_symbol.swap(index0, index);
            }
        }
    }

    /// The symbol stored at cache `index`; 0 for an out-of-range index
    /// (unreachable when N >= [`CACHE_SIZE`]).
    fn get_symbol(&self, index: usize) -> u8 {
        self.to_symbol.get(index).copied().unwrap_or(0)
    }
}

/// Adaptive symbol decoder for `N`-ary alphabets (N <= 256, typically 256),
/// combining a frequency-sorted LUT, an LRU cache, and direct bit coding.
///
/// `LIMITED` mirrors TMC13's `_limitedContextMode` template parameter (see the
/// module docs); the default `true` matches the common G-PCC configuration.
///
/// Port of TMC13 `DualLutCoder<_limitedContextMode>` (decode side).
///
/// ```
/// use gpcc::arith::ArithDecoder;
/// use gpcc::dual_lut::DualLutCoder;
///
/// let data = [0x5a, 0xc3, 0x99, 0x0f, 0x42, 0x17];
/// let mut dec = ArithDecoder::new(&data);
/// let mut coder = DualLutCoder::<256>::new();
/// let _symbol: u8 = coder.decode(&mut dec);
/// ```
pub struct DualLutCoder<const N: usize, const LIMITED: bool = true> {
    cache: FrequentSymbolCache<N>,
    adaptive_lut: FrequencySortingLut<N>,
    /// Context for the "symbol is in the LUT" flag.
    ctx_lut_hit: u16,
    /// Context for the "symbol is in the cache" flag.
    ctx_cache_hit: u16,
    /// Shared context for direct symbol bits.
    ctx_symbol_bit: u16,
    /// LUT-index contexts. `LIMITED` mode uses the first 5; full mode uses all
    /// 31 as a binary context tree (TMC13 `kNumLutContexts`).
    ctx_lut_index: [u16; 31],
}

impl<const N: usize, const LIMITED: bool> Default for DualLutCoder<N, LIMITED> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize, const LIMITED: bool> DualLutCoder<N, LIMITED> {
    /// A fresh coder with the identity LUT/cache mapping and all contexts at
    /// p = 0.5, matching TMC13's default-constructed `DualLutCoder`.
    pub fn new() -> Self {
        // TMC13: DualLutCoder<true> uses (maxUpdatePeriod, maxHistogramCount)
        // = (1024, 1024); DualLutCoder<false> uses (0x33333333, 1 << 24).
        let (max_update_period, max_histogram_count) = if LIMITED {
            (1024, 1024)
        } else {
            (0x3333_3333, 1 << 24)
        };
        Self {
            cache: FrequentSymbolCache::new(),
            adaptive_lut: FrequencySortingLut::new(max_update_period, max_histogram_count),
            ctx_lut_hit: CTX_INIT,
            ctx_cache_hit: CTX_INIT,
            ctx_symbol_bit: CTX_INIT,
            ctx_lut_index: [CTX_INIT; 31],
        }
    }

    /// Schedule a histogram reset at the LUT's next periodic update
    /// (TMC13 `resetLut`).
    pub fn reset_lut(&mut self) {
        self.adaptive_lut.reset();
    }

    /// The frequency-sorted LUT contents (most frequent symbol first).
    /// Introspection only; useful for tests and debugging.
    pub fn lut_symbols(&self) -> &[u8] {
        self.adaptive_lut.symbols()
    }

    /// Decode the 5-bit LUT index (TMC13 `decodeFrequencySortedLutIndex`).
    ///
    /// `LIMITED` mode codes the index MSB-first as a prefix: each leading zero
    /// bit gets its own adaptive context, and once a 1 is seen the remaining
    /// bits are bypass-coded. Full mode walks a 31-context binary tree, one
    /// context per node.
    fn decode_lut_index(&mut self, d: &mut ArithDecoder) -> usize {
        if LIMITED {
            let b0 = d.decode_bit(&mut self.ctx_lut_index[0]);
            let (b1, b2, b3, b4);
            if b0 == 1 {
                b1 = d.decode_bypass();
                b2 = d.decode_bypass();
                b3 = d.decode_bypass();
                b4 = d.decode_bypass();
            } else {
                b1 = d.decode_bit(&mut self.ctx_lut_index[1]);
                if b1 == 1 {
                    b2 = d.decode_bypass();
                    b3 = d.decode_bypass();
                    b4 = d.decode_bypass();
                } else {
                    b2 = d.decode_bit(&mut self.ctx_lut_index[2]);
                    if b2 == 1 {
                        b3 = d.decode_bypass();
                        b4 = d.decode_bypass();
                    } else {
                        b3 = d.decode_bit(&mut self.ctx_lut_index[3]);
                        b4 = d.decode_bit(&mut self.ctx_lut_index[4]);
                    }
                }
            }
            usize::from(b0) << 4
                | usize::from(b1) << 3
                | usize::from(b2) << 2
                | usize::from(b3) << 1
                | usize::from(b4)
        } else {
            // Binary context tree: node offsets 0, 1, 3, 7, 15 (TMC13).
            let mut index = 0usize;
            for offset in [0usize, 1, 3, 7, 15] {
                // offset + index <= 15 + 15 = 30, within the 31 contexts.
                let slot = (offset + index).min(30);
                index = index << 1 | usize::from(d.decode_bit(&mut self.ctx_lut_index[slot]));
            }
            index
        }
    }

    /// Decode one symbol (TMC13 `DualLutCoder::decode`).
    ///
    /// Tries the LUT, then the cache, then direct 8-bit coding; afterwards the
    /// cache (on a LUT miss) and the LUT histogram are updated with the
    /// decoded symbol, keeping decoder state in lockstep with the encoder.
    ///
    /// The result is always within the `N`-symbol alphabet: for N < 256 a
    /// directly-coded symbol is reduced modulo N (only reachable on invalid
    /// streams; a no-op for N = 256, where this matches TMC13 exactly).
    pub fn decode(&mut self, d: &mut ArithDecoder) -> u8 {
        if N == 0 {
            return 0;
        }

        let in_lut = d.decode_bit(&mut self.ctx_lut_hit) == 1;
        let symbol: usize;
        if in_lut {
            let index = self.decode_lut_index(d);
            symbol = usize::from(self.adaptive_lut.get_symbol(index));
        } else {
            let in_cache = d.decode_bit(&mut self.ctx_cache_hit) == 1;
            if in_cache {
                let mut index = 0usize;
                for i in 0..CACHE_LOG2 {
                    index |= usize::from(d.decode_bypass()) << i;
                }
                symbol = usize::from(self.cache.get_symbol(index));
            } else {
                let mut direct = 0usize;
                for i in 0..8 {
                    direct |= usize::from(d.decode_bit(&mut self.ctx_symbol_bit)) << i;
                }
                symbol = direct % N;
            }
            self.cache.push_symbol(symbol);
        }

        self.adaptive_lut.push_symbol(symbol);
        symbol as u8
    }
}

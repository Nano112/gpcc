# gpcc

Entropy coding primitives for **MPEG G-PCC** (Geometry-based Point Cloud
Compression, ISO/IEC 23090-9), in safe Rust.

```toml
[dependencies]
gpcc = "0.1"
```

One runtime dependency (`thiserror`). No `unsafe`. No panics in library code.

## What's here

| module | what it is |
|---|---|
| `arith` | Schroedinger/Dirac binary arithmetic decoder — the entropy engine G-PCC is built on |
| `golomb` | exp-Golomb binarisation: unsigned, signed, and multi-context forms |
| `dual_lut` | `DualLutCoder` — a 32-entry frequency-sorted LUT over a 16-entry LRU cache |
| `PROB_UPDATE_LUT` | the 512-entry interleaved probability-update table |

This crate is the entropy layer only. It does not implement octree geometry
coding, RAHT attribute transforms, or any container format — it gives you the
primitives those are built from.

## Example

```rust
use gpcc::arith::ArithDecoder;
use gpcc::golomb::decode_exp_golomb;

let mut dec = ArithDecoder::new(&stream);

// An adaptive context is just a u16 probability state.
let mut ctx: u16 = 0x8000;
let value = decode_exp_golomb(&mut dec, 0, &mut ctx)?;

// Equiprobable bits bypass the model entirely.
let raw_bit = dec.decode_bypass();
# Ok::<(), gpcc::Error>(())
```

## Correctness

The arithmetic decoder is **validated bit-exact against real-world encoded
streams**, not just against itself: decoding the raw-coded leading values of 108
independently produced bitstreams reproduces the reference decoder's integers
exactly, on every one. Nothing about the coder, the probability table or the
initialisation can be even slightly wrong and still do that.

`DualLutCoder` is a bit-exact port of the reference implementation for the
256-symbol alphabet, including bit orders, push ordering, the tie-break sort key,
histogram halving and update-period growth. Deliberate divergences (a full sort
where the reference uses `nth_element`, saturating arithmetic where it relies on
integer wrap, and clamps replacing asserts so hostile input cannot panic) are
documented in the source and are unreachable or behaviour-preserving.

Two stream-initialisation variants are supported, `Init::Schro` (two preloaded
bytes) and `Init::Preload4` (four). They are not interchangeable; real streams
use the former, which is the default.

## Licence

**MIT AND BSD-3-Clause.**

Every file is MIT except `src/dual_lut.rs`, which derives from ISO/IEC's
BSD-3-Clause reference code. Each source file carries an SPDX header; see
[`THIRD-PARTY-NOTICES.md`](THIRD-PARTY-NOTICES.md) for the full picture,
including the explicit election of MIT from Schroedinger's four-licence offer.

BSD-3-Clause is permissive — it asks you to keep a notice, nothing more.

### Patents

Neither upstream grants patent rights, and both say so explicitly. G-PCC is a
standardised codec with an active patent licensing programme, and implementing a
decoder for a standard may read on patents regardless of the copyright licence on
the source. This crate grants no patent rights and cannot.

Offered for research and interoperability. Take your own advice before shipping
commercially.

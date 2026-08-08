# Third-party notices

`gpcc` is distributed under **MIT AND BSD-3-Clause**. Every file except one is
MIT; `src/dual_lut.rs` is BSD-3-Clause. Each source file carries an
`SPDX-License-Identifier` header stating which applies.

---

## Schroedinger — MIT (elected)

`src/arith.rs` and `src/lut.rs` are derived from `schroarith.h` / `schroarith.c`
in the Schroedinger library, as vendored under `dependencies/schroedinger/` in
MPEG's `mpeg-pcc-tmc13`.

Schroedinger's `COPYING` states:

> This library is licensed under 4 different licenses and you can choose to use
> it under the terms of any one of them. The four licenses are the MPL 1.1, the
> LGPL, the GPL and the MIT license.

**We elect the MIT license.** This election is recorded here deliberately, so
downstream users need not re-derive it.

Specifically derived:

- `_schro_arith_decode_bit` — the binary arithmetic decoding step
- `schro_arith_decode_init` — decoder initialisation
- `lut_interleaved[512]` — the probability-update table, transcribed verbatim

---

## MPEG TMC13 (ISO/IEC) — BSD-3-Clause

`src/dual_lut.rs` is a port of the decode side of `tmc3/DualLutCoder.h` and
`tmc3/DualLutCoder.cpp` from `mpeg-pcc-tmc13`.

```
The copyright in this software is being made available under the BSD
Licence, included below.  This software may be subject to other third
party and contributor rights, including patent rights, and no such
rights are granted under this licence.

Copyright (c) 2017-2018, ISO/IEC
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

* Redistributions of source code must retain the above copyright
  notice, this list of conditions and the following disclaimer.

* Redistributions in binary form must reproduce the above copyright
  notice, this list of conditions and the following disclaimer in the
  documentation and/or other materials provided with the distribution.

* Neither the name of the ISO/IEC nor the names of its contributors
  may be used to endorse or promote products derived from this
  software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE
LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
POSSIBILITY OF SUCH DAMAGE.
```

---

## Exp-Golomb binarisation

`src/golomb.rs` implements exp-Golomb binarisation, a standard construction that
predates this codebase and appears throughout video-coding literature (H.264 /
HEVC CABAC and elsewhere). TMC13's `tmc3/entropyutils.h` was consulted as a
reference implementation. The algorithm itself is not subject to copyright;
the attribution is recorded as a courtesy.

---

## Patents

Neither upstream licence grants patent rights, and both say so explicitly.
TMC13's header states that the software "may be subject to other third party and
contributor rights, including patent rights, and no such rights are granted
under this licence." Schroedinger ships a `LEGAL` file describing known patents.

G-PCC (ISO/IEC 23090-9) is a standardised codec with an active patent licensing
programme. Implementing a decoder for a standard may read on patents regardless
of the copyright licence on the source. Nothing in this crate grants, or can
grant, any patent rights.

This crate is offered for research and interoperability. If you intend to ship it
commercially, take your own advice on patents.

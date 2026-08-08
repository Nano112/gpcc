// SPDX-License-Identifier: MIT
//! MPEG G-PCC (ISO/IEC 23090-9) entropy coding primitives.
//!
//! Vendor-neutral: this crate implements the standard and nothing else. It knows
//! nothing about any particular container or application format.
//!
//! Provides the Schroedinger/Dirac binary arithmetic codec, exp-Golomb
//! binarisation, and the DualLutCoder symbol coder — the entropy layer shared by
//! every G-PCC profile.

pub mod arith;
pub mod dual_lut;
pub mod error;
pub mod golomb;
mod lut;

pub use error::{Error, Result};
pub use lut::PROB_UPDATE_LUT;

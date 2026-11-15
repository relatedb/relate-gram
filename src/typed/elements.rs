//! The elements of an `embedding` payload: bytes to f64 and back, per dtype (ADR-014 section 2.4).
//!
//! Relate stores a vector's elements as floats already rounded to the vector's kind, so widening
//! a payload to f64 is exact for every dtype and narrowing a stored element back is exact too.
//! The 16-bit formats are converted here by hand: gram-value carries the bytes and does not decode
//! them, and the crate takes no dependency for it.

use gram_value::tags::EmbeddingDtype;

/// The elements of a little-endian payload as f64.
pub fn widen(dtype: EmbeddingDtype, bytes: &[u8]) -> Vec<f64> {
    match dtype {
        EmbeddingDtype::F64 => bytes
            .chunks_exact(8)
            .map(|c| f64::from_le_bytes(c.try_into().expect("8 bytes")))
            .collect(),
        EmbeddingDtype::F32 => bytes
            .chunks_exact(4)
            .map(|c| f64::from(f32::from_le_bytes(c.try_into().expect("4 bytes"))))
            .collect(),
        EmbeddingDtype::F16 => bytes
            .chunks_exact(2)
            .map(|c| f16_to_f64(u16::from_le_bytes([c[0], c[1]])))
            .collect(),
        EmbeddingDtype::BF16 => bytes
            .chunks_exact(2)
            .map(|c| bf16_to_f64(u16::from_le_bytes([c[0], c[1]])))
            .collect(),
    }
}

/// The little-endian payload of `values` in `dtype`, each rounded to nearest, ties to even.
pub fn narrow(dtype: EmbeddingDtype, values: &[f64]) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * dtype.width());
    for &v in values {
        match dtype {
            EmbeddingDtype::F64 => out.extend_from_slice(&v.to_le_bytes()),
            EmbeddingDtype::F32 => out.extend_from_slice(&(v as f32).to_le_bytes()),
            EmbeddingDtype::F16 => out.extend_from_slice(&f64_to_f16(v).to_le_bytes()),
            EmbeddingDtype::BF16 => out.extend_from_slice(&f64_to_bf16(v).to_le_bytes()),
        }
    }
    out
}

fn bf16_to_f64(bits: u16) -> f64 {
    f64::from(f32::from_bits(u32::from(bits) << 16))
}

fn f64_to_bf16(v: f64) -> u16 {
    let bits = (v as f32).to_bits();
    if (bits & 0x7fff_ffff) > 0x7f80_0000 {
        // NaN: keep it a NaN after the shift.
        return ((bits >> 16) | 0x0040) as u16;
    }
    let rounded = bits.wrapping_add(0x7fff + ((bits >> 16) & 1));
    (rounded >> 16) as u16
}

fn f16_to_f64(bits: u16) -> f64 {
    let sign = if bits & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exp = i32::from((bits >> 10) & 0x1f);
    let frac = f64::from(bits & 0x03ff);
    match exp {
        0 => sign * frac * 2f64.powi(-24),
        31 if frac == 0.0 => sign * f64::INFINITY,
        31 => f64::NAN,
        _ => sign * (1.0 + frac / 1024.0) * 2f64.powi(exp - 15),
    }
}

fn f64_to_f16(v: f64) -> u16 {
    let sign: u16 = if v.is_sign_negative() { 0x8000 } else { 0 };
    if v.is_nan() {
        return sign | 0x7e00;
    }
    let a = v.abs();
    if a == 0.0 {
        return sign;
    }
    if a.is_infinite() || a >= 65520.0 {
        return sign | 0x7c00;
    }
    // The exponent of `a` as a power of two, from its bits (a is finite and non-zero).
    let e = ((a.to_bits() >> 52) & 0x7ff) as i32 - 1023;
    if e < -14 {
        // Subnormal: a multiple of 2^-24, 0 to 1024 of them (1024 is the smallest normal).
        let m = (a * 2f64.powi(24)).round_ties_even() as u16;
        return sign | m;
    }
    let mut exp = e;
    let mut mant = (a / 2f64.powi(exp - 10)).round_ties_even() as u32;
    if mant == 2048 {
        mant = 1024;
        exp += 1;
    }
    if exp > 15 {
        return sign | 0x7c00;
    }
    sign | (((exp + 15) as u16) << 10) | (mant - 1024) as u16
}

#[cfg(test)]
mod tests {
    use super::*;
    use EmbeddingDtype::*;

    fn bytes(parts: &[&[u8]]) -> Vec<u8> {
        parts.concat()
    }

    #[test]
    fn f32_and_f64_round_trip_their_values() {
        let v = [1.0, -2.0, 0.1f32 as f64, -0.0];
        for d in [F32, F64] {
            let b = narrow(d, &v);
            assert_eq!(b.len(), v.len() * d.width());
            let w = widen(d, &b);
            for (a, b) in v.iter().zip(&w) {
                assert_eq!(a.to_bits(), b.to_bits(), "{d:?}");
            }
        }
    }

    #[test]
    fn bf16_is_the_top_half_of_an_f32() {
        assert_eq!(
            narrow(BF16, &[1.0, -2.0]),
            bytes(&[&[0x80, 0x3f], &[0x00, 0xc0]])
        );
        assert_eq!(widen(BF16, &[0x80, 0x3f, 0x00, 0xc0]), vec![1.0, -2.0]);
    }

    #[test]
    fn f16_matches_the_reference_corpus_values() {
        // The corpus case `tag-embedding-ref-f16`: 1.0, 1.5, 2.0, 3.0, 6.0.
        let payload = [0x00, 0x3c, 0x00, 0x3e, 0x00, 0x40, 0x00, 0x42, 0x00, 0x46];
        assert_eq!(widen(F16, &payload), vec![1.0, 1.5, 2.0, 3.0, 6.0]);
        assert_eq!(narrow(F16, &[1.0, 1.5, 2.0, 3.0, 6.0]), payload.to_vec());
    }

    #[test]
    fn sixteen_bit_formats_keep_subnormals_and_negative_zero() {
        for d in [F16, BF16] {
            let w = widen(d, &[0x01, 0x00, 0x00, 0x80]);
            assert!(
                w[0] > 0.0 && w[0] < 1e-6,
                "{d:?} smallest subnormal {}",
                w[0]
            );
            assert!(w[1] == 0.0 && w[1].is_sign_negative());
            assert_eq!(narrow(d, &w), vec![0x01, 0x00, 0x00, 0x80]);
        }
    }

    #[test]
    fn every_finite_sixteen_bit_pattern_round_trips() {
        for d in [F16, BF16] {
            for bits in 0..=u16::MAX {
                let b = bits.to_le_bytes();
                let w = widen(d, &b);
                if !w[0].is_finite() {
                    continue;
                }
                assert_eq!(narrow(d, &w), b.to_vec(), "{d:?} {bits:#06x}");
            }
        }
    }

    #[test]
    fn non_finite_patterns_widen_to_non_finite() {
        assert!(widen(F16, &[0x00, 0x7c])[0].is_infinite());
        assert!(widen(F16, &[0x01, 0x7e])[0].is_nan());
        assert!(widen(BF16, &[0x80, 0x7f])[0].is_infinite());
        assert!(widen(F32, &[0x00, 0x00, 0xc0, 0x7f])[0].is_nan());
    }
}

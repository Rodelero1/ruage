//! Fixed-point arithmetic for the simulation.
//!
//! Invariant (SPEC §5, `docs/mechanics/determinism.md`): simulation state never contains a
//! float. The original engine desynced when the x86 FP control word changed, and a
//! reimplementation that carries floats in state inherits that class of bug while also
//! making replays machine-dependent.
//!
//! The scale (units per 1.0) is *data*, from `config/sim.toml` → `sim.fixed_point_scale`,
//! never a literal in this crate. Callers pass it in.

use core::fmt;

/// A value stored as an integer number of `1 / scale` units.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Fixed(i64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixedError {
    /// Digits below what the scale can represent. Never rounded — a silently dropped
    /// 0.0000001 is exactly the kind of plausible-but-wrong number this project refuses.
    SubPrecision {
        text: String,
        scale: i64,
    },
    OutOfRange {
        text: String,
        scale: i64,
    },
    Malformed {
        text: String,
    },
    Overflow,
}

impl fmt::Display for FixedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FixedError::SubPrecision { text, scale } => write!(
                f,
                "{text} needs more precision than scale {scale} can represent; \
                 refusing to round it"
            ),
            FixedError::OutOfRange { text, scale } => {
                write!(f, "{text} overflows i64 at scale {scale}")
            }
            FixedError::Malformed { text } => write!(f, "{text:?} is not a decimal number"),
            FixedError::Overflow => write!(f, "fixed-point arithmetic overflowed i64"),
        }
    }
}

impl std::error::Error for FixedError {}

impl Fixed {
    pub const ZERO: Self = Fixed(0);

    pub fn from_raw(raw: i64) -> Self {
        Fixed(raw)
    }

    pub fn raw(self) -> i64 {
        self.0
    }

    /// Parse an exact decimal string such as `"1.35"` into `scale` units.
    ///
    /// Exact or nothing: digits that the scale cannot represent are an error, not a
    /// rounding opportunity.
    pub fn parse(text: &str, scale: i64) -> Result<Self, FixedError> {
        if scale <= 0 {
            return Err(FixedError::OutOfRange {
                text: format!("scale {scale}"),
                scale,
            });
        }

        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(FixedError::Malformed {
                text: text.to_owned(),
            });
        }

        let (negative, body) = match trimmed.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
        };

        let (int_part, frac_part) = match body.split_once('.') {
            Some((i, f)) => (i, f),
            None => (body, ""),
        };

        // Only ASCII digits, and a fraction must have digits if the point is present.
        let digits_ok = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
        if body.contains('.') && frac_part.is_empty() {
            return Err(FixedError::Malformed {
                text: text.to_owned(),
            });
        }
        if !digits_ok(int_part) || !digits_ok(frac_part) {
            return Err(FixedError::Malformed {
                text: text.to_owned(),
            });
        }
        if int_part.is_empty() && frac_part.is_empty() {
            return Err(FixedError::Malformed {
                text: text.to_owned(),
            });
        }

        let mut numerator: i128 = 0;
        for b in int_part.bytes() {
            numerator = numerator * 10 + i128::from(b - b'0');
        }
        for b in frac_part.bytes() {
            numerator = numerator * 10 + i128::from(b - b'0');
        }
        if negative {
            numerator = -numerator;
        }

        // value = numerator / 10^d, and we need it in 1/scale units:
        // raw = numerator * scale / 10^d, which must divide exactly.
        let denominator: i128 = 10_i128
            .checked_pow(
                u32::try_from(frac_part.len()).map_err(|_| FixedError::Malformed {
                    text: text.to_owned(),
                })?,
            )
            .ok_or(FixedError::OutOfRange {
                text: text.to_owned(),
                scale,
            })?;

        let scaled = numerator
            .checked_mul(i128::from(scale))
            .ok_or(FixedError::OutOfRange {
                text: text.to_owned(),
                scale,
            })?;

        if scaled % denominator != 0 {
            return Err(FixedError::SubPrecision {
                text: text.to_owned(),
                scale,
            });
        }

        let raw = scaled / denominator;
        i64::try_from(raw)
            .map(Fixed)
            .map_err(|_| FixedError::OutOfRange {
                text: text.to_owned(),
                scale,
            })
    }

    pub fn add(self, other: Self) -> Result<Self, FixedError> {
        self.0
            .checked_add(other.0)
            .map(Fixed)
            .ok_or(FixedError::Overflow)
    }

    pub fn sub(self, other: Self) -> Result<Self, FixedError> {
        self.0
            .checked_sub(other.0)
            .map(Fixed)
            .ok_or(FixedError::Overflow)
    }

    /// Multiply two fixed values, renormalising to `scale`. Rounds half away from zero.
    pub fn mul(self, other: Self, scale: i64) -> Result<Self, FixedError> {
        if scale <= 0 {
            return Err(FixedError::OutOfRange {
                text: format!("scale {scale}"),
                scale,
            });
        }
        let product = i128::from(self.0) * i128::from(other.0);
        let rounded = div_round(product, i128::from(scale));
        i64::try_from(rounded)
            .map(Fixed)
            .map_err(|_| FixedError::Overflow)
    }

    /// Divide by an integer, rounding half away from zero. `divisor` must be positive.
    pub fn div_int(self, divisor: i64) -> Self {
        assert!(divisor > 0, "divisor must be positive, got {divisor}");
        let rounded = div_round(i128::from(self.0), i128::from(divisor));
        Fixed(i64::try_from(rounded).expect("division cannot overflow i64"))
    }
}

/// Integer division rounding half away from zero — the convention used throughout the sim.
fn div_round(numerator: i128, denominator: i128) -> i128 {
    debug_assert!(denominator > 0);
    let half = denominator / 2;
    if numerator >= 0 {
        (numerator + half) / denominator
    } else {
        (numerator - half) / denominator
    }
}

impl fmt::Debug for Fixed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fixed({})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The documented scale: 20 Hz, every distinct dat speed an exact integer.
    const S: i64 = 20_000;

    #[test]
    fn parses_exactly() {
        assert_eq!(Fixed::parse("1.35", S).unwrap().raw(), 27_000);
        assert_eq!(Fixed::parse("0.8", S).unwrap().raw(), 16_000);
        assert_eq!(Fixed::parse("2", S).unwrap().raw(), 40_000);
        assert_eq!(Fixed::parse("-0.96", S).unwrap().raw(), -19_200);
        assert_eq!(Fixed::parse("0", S).unwrap().raw(), 0);
    }

    #[test]
    fn sub_precision_is_refused_not_rounded() {
        let err = Fixed::parse("0.0000001", S).unwrap_err();
        assert!(
            matches!(err, FixedError::SubPrecision { .. }),
            "expected SubPrecision, got {err:?}"
        );
    }

    #[test]
    fn half_units_are_not_truncated_into_agreement() {
        // 0.00005 * 20000 = 1.0 exactly; 0.0000499 does not fit at this scale.
        assert_eq!(Fixed::parse("0.00005", S).unwrap().raw(), 1);
        assert!(Fixed::parse("0.000049", S).is_err());
    }

    #[test]
    fn garbage_is_rejected() {
        for bad in ["", "1.2.3", "0x10", "--1", "1,5", " "] {
            assert!(
                matches!(Fixed::parse(bad, S), Err(FixedError::Malformed { .. })),
                "{bad:?} should be Malformed"
            );
        }
    }

    #[test]
    fn dat_speeds_are_exact_and_divide_into_ticks() {
        // Every dat speed carries 2 dp, so at 20000 each is an exact integer, and the
        // per-tick displacement at 20 Hz is exact too — no accumulated drift.
        for (text, raw) in [
            ("0.8", 16_000_i64),
            ("0.96", 19_200),
            ("1.35", 27_000),
            ("1.54", 30_800),
            ("8.8", 176_000),
        ] {
            let v = Fixed::parse(text, S).unwrap();
            assert_eq!(v.raw(), raw, "speed {text}");
            assert_eq!(v.div_int(20).raw(), raw / 20, "speed {text} per tick");
        }
    }

    #[test]
    fn division_rounds_half_away_from_zero() {
        assert_eq!(Fixed::from_raw(3).div_int(2).raw(), 2);
        assert_eq!(Fixed::from_raw(-3).div_int(2).raw(), -2);
        assert_eq!(Fixed::from_raw(1).div_int(2).raw(), 1);
        assert_eq!(Fixed::from_raw(-1).div_int(2).raw(), -1);
        assert_eq!(Fixed::from_raw(9).div_int(3).raw(), 3);
    }

    #[test]
    fn multiply_is_exact_for_dat_values() {
        // 1.35 * 1.35 = 1.8225
        let a = Fixed::parse("1.35", S).unwrap();
        assert_eq!(a.mul(a, S).unwrap().raw(), 36_450);
        // 0.4 * 250 = 100
        let rate = Fixed::parse("0.4", S).unwrap();
        assert_eq!(
            rate.mul(Fixed::parse("250", S).unwrap(), S).unwrap().raw(),
            2_000_000
        );
    }

    #[test]
    fn overflow_is_an_error_not_a_wrap() {
        assert!(Fixed::from_raw(i64::MAX).add(Fixed::from_raw(1)).is_err());
        assert!(Fixed::from_raw(i64::MIN).sub(Fixed::from_raw(1)).is_err());
        let big = Fixed::from_raw(i64::MAX / 2);
        assert!(big.mul(big, S).is_err());
    }

    #[test]
    fn addition_and_subtraction_round_trip() {
        let a = Fixed::parse("1.35", S).unwrap();
        let b = Fixed::parse("0.4", S).unwrap();
        assert_eq!(a.add(b).unwrap().raw(), 35_000);
        assert_eq!(a.add(b).unwrap().sub(b).unwrap(), a);
    }
}

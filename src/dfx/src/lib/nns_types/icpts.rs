// DISCLAIMER:
// Do not modify this file arbitrarily.
// The contents are borrowed from:
// dfinity-lab/dfinity@f468897c57a5a0d4785b90c94935255d1a2f7d4c
// https://github.com/dfinity-lab/dfinity/blob/master/rs/rosetta-api/canister/src/icpts.rs

use candid::CandidType;
use core::ops::{Add, AddAssign, Sub, SubAssign};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(
    Serialize,
    Deserialize,
    CandidType,
    Clone,
    Copy,
    Hash,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
)]
pub struct ICPTs {
    /// Number of 10^-8 ICPs.
    /// Named because the equivalent part of a Bitcoin is called a Satoshi
    e8s: u64,
}

pub const DECIMAL_PLACES: u32 = 8;
/// How many times can ICPs be divided
pub const ICP_SUBDIVIDABLE_BY: u64 = 100_000_000;

pub const TRANSACTION_FEE: ICPTs = ICPTs { e8s: 137 };

#[allow(dead_code)]
pub const MIN_BURN_AMOUNT: ICPTs = TRANSACTION_FEE;

#[allow(dead_code)]
impl ICPTs {
    /// The maximum value of this construct is 2^64-1 Doms or Roughly 184
    /// Billion ICPTs
    pub const MAX: Self = ICPTs { e8s: u64::MAX };

    /// Construct a new instance of ICPTs.
    /// This function will not allow you use more than 1 ICPTs worth of Doms.
    pub fn new(icpt: u64, e8s: u64) -> Result<Self, String> {
        static CONSTRUCTION_FAILED: &str =
            "Constructing ICP failed because the underlying u64 overflowed";

        let icp_part = icpt
            .checked_mul(ICP_SUBDIVIDABLE_BY)
            .ok_or_else(|| CONSTRUCTION_FAILED.to_string())?;
        if e8s >= ICP_SUBDIVIDABLE_BY {
            return Err(format!(
                "You've added too many Doms, make sure there are less than {}",
                ICP_SUBDIVIDABLE_BY
            ));
        }
        let e8s = icp_part
            .checked_add(e8s)
            .ok_or_else(|| CONSTRUCTION_FAILED.to_string())?;
        Ok(Self { e8s })
    }

    pub const ZERO: Self = ICPTs { e8s: 0 };

    /// ```
    /// # use ledger_canister::ICPTs;
    /// let icpt = ICPTs::from_icpts(12).unwrap();
    /// assert_eq!(icpt.unpack(), (12, 0))
    /// ```
    pub fn from_icpts(icp: u64) -> Result<Self, String> {
        Self::new(icp, 0)
    }

    /// Construct ICPTs from Doms, 10E8 Doms == 1 ICP
    /// ```
    /// # use ledger_canister::ICPTs;
    /// let icpt = ICPTs::from_e8s(1200000200);
    /// assert_eq!(icpt.unpack(), (12, 200))
    /// ```
    pub const fn from_e8s(e8s: u64) -> Self {
        ICPTs { e8s }
    }

    /// Gets the total number of whole ICPTs
    /// ```
    /// # use ledger_canister::ICPTs;
    /// let icpt = ICPTs::new(12, 200).unwrap();
    /// assert_eq!(icpt.get_icpts(), 12)
    /// ```
    pub fn get_icpts(self) -> u64 {
        self.e8s / ICP_SUBDIVIDABLE_BY
    }

    /// Gets the total number of Doms
    /// ```
    /// # use ledger_canister::ICPTs;
    /// let icpt = ICPTs::new(12, 200).unwrap();
    /// assert_eq!(icpt.get_e8s(), 1200000200)
    /// ```
    pub fn get_e8s(self) -> u64 {
        self.e8s
    }

    /// Gets the total number of Doms not part of a whole ICPT
    /// The returned amount is always in the half-open interval [0, 1 ICP).
    /// ```
    /// # use ledger_canister::ICPTs;
    /// let icpt = ICPTs::new(12, 200).unwrap();
    /// assert_eq!(icpt.get_remainder_e8s(), 200)
    /// ```
    pub fn get_remainder_e8s(self) -> u64 {
        self.e8s % ICP_SUBDIVIDABLE_BY
    }

    /// This returns the number of ICPTs and Doms
    /// ```
    /// # use ledger_canister::ICPTs;
    /// let icpt = ICPTs::new(12, 200).unwrap();
    /// assert_eq!(icpt.unpack(), (12, 200))
    /// ```
    pub fn unpack(self) -> (u64, u64) {
        (self.get_icpts(), self.get_remainder_e8s())
    }
}

impl Add for ICPTs {
    type Output = Result<Self, String>;

    /// This returns a result, in normal operation this should always return Ok
    /// because of the cap in the total number of ICP, but when dealing with
    /// money it's better to be safe than sorry
    fn add(self, other: Self) -> Self::Output {
        let e8s = self.e8s.checked_add(other.e8s).ok_or_else(|| {
            format!(
                "Add ICP {} + {} failed because the underlying u64 overflowed",
                self.e8s, other.e8s
            )
        })?;
        Ok(Self { e8s })
    }
}

impl AddAssign for ICPTs {
    fn add_assign(&mut self, other: Self) {
        *self = (*self + other).expect("+= panicked");
    }
}

impl Sub for ICPTs {
    type Output = Result<Self, String>;

    fn sub(self, other: Self) -> Self::Output {
        let e8s = self.e8s.checked_sub(other.e8s).ok_or_else(|| {
            format!(
                "Subtracting ICP {} - {} failed because the underlying u64 underflowed",
                self.e8s, other.e8s
            )
        })?;
        Ok(Self { e8s })
    }
}

impl SubAssign for ICPTs {
    fn sub_assign(&mut self, other: Self) {
        *self = (*self - other).expect("-= panicked");
    }
}

/// ```
/// # use ledger_canister::ICPTs;
/// let icpt = ICPTs::new(12, 200).unwrap();
/// let s = format!("{}", icpt);
/// assert_eq!(&s[..], "12.00000200 ICP")
/// ```
impl fmt::Display for ICPTs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{:08} ICP",
            self.get_icpts(),
            self.get_remainder_e8s()
        )
    }
}

impl FromStr for ICPTs {
    type Err = String;

    fn from_str(s: &str) -> Result<ICPTs, String> {
        match Decimal::from_str(s) {
            Ok(amount) => {
                if amount.scale() > DECIMAL_PLACES {
                    return Err("Doms only go to e8".to_string());
                }
                let icpts = match amount.trunc().to_string().parse::<u64>() {
                    Ok(v) => v,
                    Err(e) => return Err(format!("{}", e)),
                };
                let e8s = match amount.fract().to_string().as_str() {
                    "0" => 0_u64,
                    e8s => {
                        let e8s = &e8s.to_string()[2..e8s.to_string().len()];
                        let amount = e8s.chars().enumerate().fold(0, |amount, (idx, val)| {
                            amount
                                + (10_u64.pow(DECIMAL_PLACES - 1 - (idx as u32))
                                    * (val.to_digit(10).unwrap() as u64))
                        });
                        amount as u64
                    }
                };
                ICPTs::new(icpts, e8s)
            }
            Err(e) => Err(format!("Decimal conversion error: {}", e)),
        }
    }
}

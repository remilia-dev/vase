// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::convert::{
    TryFrom,
    TryInto,
};

use crate::util::create_intos;

/// An enum that represents either an i64 or an u64.
#[create_intos]
#[derive(Clone, Debug)]
pub enum Sign {
    Signed(i64),
    Unsigned(u64),
}

impl Sign {
    /// Returns whether this value is a signed value.
    pub fn is_signed(&self) -> bool {
        matches!(*self, Self::Signed(..))
    }
    /// Returns whether this value is an unsigned value.
    pub fn is_unsigned(&self) -> bool {
        matches!(*self, Self::Unsigned(..))
    }
    /// Returns whether the contained value is zero.
    pub fn is_zero(&self) -> bool {
        match *self {
            Self::Signed(i) => i == 0,
            Self::Unsigned(u) => u == 0,
        }
    }
    /// If this value is signed, return it.
    /// If the value is unsigned, it will *attempt* to convert it to signed.
    pub fn signed(&self) -> Option<i64> {
        match *self {
            Self::Signed(i) => Some(i),
            Self::Unsigned(u) => u.try_into().ok(),
        }
    }
    /// If this value is unsigned, return it.
    /// If the value is signed, it will *attempt* to convert it to unsigned.
    pub fn unsigned(&self) -> Option<u64> {
        match *self {
            Self::Signed(i) => i.try_into().ok(),
            Self::Unsigned(u) => Some(u),
        }
    }
    /// Returns the value contained inside as an i64 and a flag if the value was wrapped.
    ///
    /// The wrapped flag can only be true if the value contained was unsigned.
    pub fn wrapped_signed(&self) -> (i64, bool) {
        match *self {
            Self::Signed(i) => (i, false),
            Self::Unsigned(u) => match u.try_into() {
                Ok(i) => (i, false),
                Err(..) => (u as i64, true),
            },
        }
    }
    /// Returns the value contained inside as an u64 and a flag if the value was wrapped.
    ///
    /// The wrapped flag can only be true if the value contained was signed.
    pub fn wrapped_unsigned(&self) -> (u64, bool) {
        match *self {
            Self::Signed(i) => match i.try_into() {
                Ok(u) => (u, false),
                Err(..) => (i as u64, true),
            },
            Self::Unsigned(u) => (u, false),
        }
    }
}

impl TryFrom<Sign> for u64 {
    type Error = i64;

    fn try_from(value: Sign) -> Result<Self, Self::Error> {
        match value {
            Sign::Signed(i) => i.try_into().map_err(|_| i),
            Sign::Unsigned(u) => Ok(u),
        }
    }
}

impl TryFrom<Sign> for i64 {
    type Error = u64;

    fn try_from(value: Sign) -> Result<Self, Self::Error> {
        match value {
            Sign::Signed(i) => Ok(i),
            Sign::Unsigned(u) => u.try_into().map_err(|_| u),
        }
    }
}

impl std::ops::Not for Sign {
    type Output = Self;
    /// Since both u64 and i64 implement Not, Sign implements it as well.
    fn not(self) -> Self::Output {
        match self {
            Self::Signed(i) => (!i).into(),
            Self::Unsigned(u) => (!u).into(),
        }
    }
}

// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::convert::TryInto;
use std::ops::*;

/// A trait that represents a numerical type.
///
/// It is implemented for all signed, unsigned, and binary floating point
/// types rust provides (with the exception of i8).
///
/// This trait cannot be implemented by other types.
pub trait Number:
    Copy
    + std::fmt::Debug
    + std::fmt::Display
    + Sized
    + PartialEq
    + PartialOrd
    + From<u8>
    + Add<Output = Self>
    + AddAssign
    + Sub<Output = Self>
    + SubAssign
    + Mul<Output = Self>
    + MulAssign
    + Div<Output = Self>
    + DivAssign
    + Rem<Output = Self>
    + RemAssign
    + private::Sealed
{
}
impl Number for u8 {}
impl Number for u16 {}
impl Number for u32 {}
impl Number for u64 {}
impl Number for u128 {}
impl Number for usize {}
impl Number for i16 {}
impl Number for i32 {}
impl Number for i64 {}
impl Number for i128 {}
impl Number for isize {}
impl Number for f32 {}
impl Number for f64 {}

/// A type that represents a real (non-integer) number type.
///
/// This is implemented for 'f32' and 'f64'.
/// # Note
/// This trait may be implemented by non-binary-floating-point types in the future.
pub trait Real: Number + Neg {
    fn powi(self, rhs: i32) -> Self;
    fn is_finite(self) -> bool;
}
impl Real for f32 {
    fn powi(self, rhs: i32) -> Self {
        Self::powi(self, rhs)
    }

    fn is_finite(self) -> bool {
        Self::is_finite(self)
    }
}
impl Real for f64 {
    fn powi(self, rhs: i32) -> Self {
        Self::powi(self, rhs)
    }

    fn is_finite(self) -> bool {
        Self::is_finite(self)
    }
}

/// A type that represents an integer number (unsigned or signed).
///
/// This is implemented for all integer types rust provides (except i8).
pub trait Integer:
    Number
    + Eq
    + Ord
    + Not<Output = Self>
    + BitAnd<Output = Self>
    + BitAndAssign
    + BitOr<Output = Self>
    + BitOrAssign
    + BitXor<Output = Self>
    + BitXorAssign
    + Shl<Output = Self>
    + ShlAssign
    + Shr<Output = Self>
    + ShrAssign
{
    fn overflowing_neg(self) -> (Self, bool);
    fn overflowing_add(self, rhs: Self) -> (Self, bool);
    fn overflowing_sub(self, rhs: Self) -> (Self, bool);
    fn overflowing_mul(self, rhs: Self) -> (Self, bool);
    fn overflowing_pow(self, rhs: u32) -> (Self, bool);
    fn checked_div(self, rhs: Self) -> Option<Self>;
    fn checked_rem(self, rhs: Self) -> Option<Self>;
    fn checked_shl(self, rhs: Self) -> Option<Self>;
    fn checked_shr(self, rhs: Self) -> Option<Self>;
}
/// TODO: It would be nice if the below could use a macro rather than copy-paste.
impl Integer for u8 {
    fn overflowing_neg(self) -> (Self, bool) {
        Self::overflowing_neg(self)
    }

    fn overflowing_add(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_add(self, rhs)
    }

    fn overflowing_sub(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_sub(self, rhs)
    }

    fn overflowing_mul(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_mul(self, rhs)
    }

    fn overflowing_pow(self, rhs: u32) -> (Self, bool) {
        Self::overflowing_pow(self, rhs)
    }

    fn checked_div(self, rhs: Self) -> Option<Self> {
        Self::checked_div(self, rhs)
    }

    fn checked_rem(self, rhs: Self) -> Option<Self> {
        Self::checked_rem(self, rhs)
    }

    fn checked_shl(self, rhs: Self) -> Option<Self> {
        Self::checked_shl(self, rhs.try_into().ok()?)
    }

    fn checked_shr(self, rhs: Self) -> Option<Self> {
        Self::checked_shr(self, rhs.try_into().ok()?)
    }
}
impl Integer for u16 {
    fn overflowing_neg(self) -> (Self, bool) {
        Self::overflowing_neg(self)
    }

    fn overflowing_add(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_add(self, rhs)
    }

    fn overflowing_sub(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_sub(self, rhs)
    }

    fn overflowing_mul(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_mul(self, rhs)
    }

    fn overflowing_pow(self, rhs: u32) -> (Self, bool) {
        Self::overflowing_pow(self, rhs)
    }

    fn checked_div(self, rhs: Self) -> Option<Self> {
        Self::checked_div(self, rhs)
    }

    fn checked_rem(self, rhs: Self) -> Option<Self> {
        Self::checked_rem(self, rhs)
    }

    fn checked_shl(self, rhs: Self) -> Option<Self> {
        Self::checked_shl(self, rhs.try_into().ok()?)
    }

    fn checked_shr(self, rhs: Self) -> Option<Self> {
        Self::checked_shr(self, rhs.try_into().ok()?)
    }
}
impl Integer for u32 {
    fn overflowing_neg(self) -> (Self, bool) {
        Self::overflowing_neg(self)
    }

    fn overflowing_add(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_add(self, rhs)
    }

    fn overflowing_sub(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_sub(self, rhs)
    }

    fn overflowing_mul(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_mul(self, rhs)
    }

    fn overflowing_pow(self, rhs: u32) -> (Self, bool) {
        Self::overflowing_pow(self, rhs)
    }

    fn checked_div(self, rhs: Self) -> Option<Self> {
        Self::checked_div(self, rhs)
    }

    fn checked_rem(self, rhs: Self) -> Option<Self> {
        Self::checked_rem(self, rhs)
    }

    fn checked_shl(self, rhs: Self) -> Option<Self> {
        Self::checked_shl(self, rhs)
    }

    fn checked_shr(self, rhs: Self) -> Option<Self> {
        Self::checked_shr(self, rhs)
    }
}
impl Integer for u64 {
    fn overflowing_neg(self) -> (Self, bool) {
        Self::overflowing_neg(self)
    }

    fn overflowing_add(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_add(self, rhs)
    }

    fn overflowing_sub(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_sub(self, rhs)
    }

    fn overflowing_mul(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_mul(self, rhs)
    }

    fn overflowing_pow(self, rhs: u32) -> (Self, bool) {
        Self::overflowing_pow(self, rhs)
    }

    fn checked_div(self, rhs: Self) -> Option<Self> {
        Self::checked_div(self, rhs)
    }

    fn checked_rem(self, rhs: Self) -> Option<Self> {
        Self::checked_rem(self, rhs)
    }

    fn checked_shl(self, rhs: Self) -> Option<Self> {
        Self::checked_shl(self, rhs.try_into().ok()?)
    }

    fn checked_shr(self, rhs: Self) -> Option<Self> {
        Self::checked_shr(self, rhs.try_into().ok()?)
    }
}
impl Integer for u128 {
    fn overflowing_neg(self) -> (Self, bool) {
        Self::overflowing_neg(self)
    }

    fn overflowing_add(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_add(self, rhs)
    }

    fn overflowing_sub(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_sub(self, rhs)
    }

    fn overflowing_mul(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_mul(self, rhs)
    }

    fn overflowing_pow(self, rhs: u32) -> (Self, bool) {
        Self::overflowing_pow(self, rhs)
    }

    fn checked_div(self, rhs: Self) -> Option<Self> {
        Self::checked_div(self, rhs)
    }

    fn checked_rem(self, rhs: Self) -> Option<Self> {
        Self::checked_rem(self, rhs)
    }

    fn checked_shl(self, rhs: Self) -> Option<Self> {
        Self::checked_shl(self, rhs.try_into().ok()?)
    }

    fn checked_shr(self, rhs: Self) -> Option<Self> {
        Self::checked_shr(self, rhs.try_into().ok()?)
    }
}
impl Integer for usize {
    fn overflowing_neg(self) -> (Self, bool) {
        Self::overflowing_neg(self)
    }

    fn overflowing_add(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_add(self, rhs)
    }

    fn overflowing_sub(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_sub(self, rhs)
    }

    fn overflowing_mul(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_mul(self, rhs)
    }

    fn overflowing_pow(self, rhs: u32) -> (Self, bool) {
        Self::overflowing_pow(self, rhs)
    }

    fn checked_div(self, rhs: Self) -> Option<Self> {
        Self::checked_div(self, rhs)
    }

    fn checked_rem(self, rhs: Self) -> Option<Self> {
        Self::checked_rem(self, rhs)
    }

    fn checked_shl(self, rhs: Self) -> Option<Self> {
        Self::checked_shl(self, rhs.try_into().ok()?)
    }

    fn checked_shr(self, rhs: Self) -> Option<Self> {
        Self::checked_shr(self, rhs.try_into().ok()?)
    }
}
impl Integer for i16 {
    fn overflowing_neg(self) -> (Self, bool) {
        Self::overflowing_neg(self)
    }

    fn overflowing_add(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_add(self, rhs)
    }

    fn overflowing_sub(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_sub(self, rhs)
    }

    fn overflowing_mul(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_mul(self, rhs)
    }

    fn overflowing_pow(self, rhs: u32) -> (Self, bool) {
        Self::overflowing_pow(self, rhs)
    }

    fn checked_div(self, rhs: Self) -> Option<Self> {
        Self::checked_div(self, rhs)
    }

    fn checked_rem(self, rhs: Self) -> Option<Self> {
        Self::checked_rem(self, rhs)
    }

    fn checked_shl(self, rhs: Self) -> Option<Self> {
        Self::checked_shl(self, rhs.try_into().ok()?)
    }

    fn checked_shr(self, rhs: Self) -> Option<Self> {
        Self::checked_shr(self, rhs.try_into().ok()?)
    }
}
impl Integer for i32 {
    fn overflowing_neg(self) -> (Self, bool) {
        Self::overflowing_neg(self)
    }

    fn overflowing_add(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_add(self, rhs)
    }

    fn overflowing_sub(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_sub(self, rhs)
    }

    fn overflowing_mul(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_mul(self, rhs)
    }

    fn overflowing_pow(self, rhs: u32) -> (Self, bool) {
        Self::overflowing_pow(self, rhs)
    }

    fn checked_div(self, rhs: Self) -> Option<Self> {
        Self::checked_div(self, rhs)
    }

    fn checked_rem(self, rhs: Self) -> Option<Self> {
        Self::checked_rem(self, rhs)
    }

    fn checked_shl(self, rhs: Self) -> Option<Self> {
        Self::checked_shl(self, rhs.try_into().ok()?)
    }

    fn checked_shr(self, rhs: Self) -> Option<Self> {
        Self::checked_shr(self, rhs.try_into().ok()?)
    }
}
impl Integer for i64 {
    fn overflowing_neg(self) -> (Self, bool) {
        Self::overflowing_neg(self)
    }

    fn overflowing_add(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_add(self, rhs)
    }

    fn overflowing_sub(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_sub(self, rhs)
    }

    fn overflowing_mul(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_mul(self, rhs)
    }

    fn overflowing_pow(self, rhs: u32) -> (Self, bool) {
        Self::overflowing_pow(self, rhs)
    }

    fn checked_div(self, rhs: Self) -> Option<Self> {
        Self::checked_div(self, rhs)
    }

    fn checked_rem(self, rhs: Self) -> Option<Self> {
        Self::checked_rem(self, rhs)
    }

    fn checked_shl(self, rhs: Self) -> Option<Self> {
        Self::checked_shl(self, rhs.try_into().ok()?)
    }

    fn checked_shr(self, rhs: Self) -> Option<Self> {
        Self::checked_shr(self, rhs.try_into().ok()?)
    }
}
impl Integer for i128 {
    fn overflowing_neg(self) -> (Self, bool) {
        Self::overflowing_neg(self)
    }

    fn overflowing_add(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_add(self, rhs)
    }

    fn overflowing_sub(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_sub(self, rhs)
    }

    fn overflowing_mul(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_mul(self, rhs)
    }

    fn overflowing_pow(self, rhs: u32) -> (Self, bool) {
        Self::overflowing_pow(self, rhs)
    }

    fn checked_div(self, rhs: Self) -> Option<Self> {
        Self::checked_div(self, rhs)
    }

    fn checked_rem(self, rhs: Self) -> Option<Self> {
        Self::checked_rem(self, rhs)
    }

    fn checked_shl(self, rhs: Self) -> Option<Self> {
        Self::checked_shl(self, rhs.try_into().ok()?)
    }

    fn checked_shr(self, rhs: Self) -> Option<Self> {
        Self::checked_shr(self, rhs.try_into().ok()?)
    }
}
impl Integer for isize {
    fn overflowing_neg(self) -> (Self, bool) {
        Self::overflowing_neg(self)
    }

    fn overflowing_add(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_add(self, rhs)
    }

    fn overflowing_sub(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_sub(self, rhs)
    }

    fn overflowing_mul(self, rhs: Self) -> (Self, bool) {
        Self::overflowing_mul(self, rhs)
    }

    fn overflowing_pow(self, rhs: u32) -> (Self, bool) {
        Self::overflowing_pow(self, rhs)
    }

    fn checked_div(self, rhs: Self) -> Option<Self> {
        Self::checked_div(self, rhs)
    }

    fn checked_rem(self, rhs: Self) -> Option<Self> {
        Self::checked_rem(self, rhs)
    }

    fn checked_shl(self, rhs: Self) -> Option<Self> {
        Self::checked_shl(self, rhs.try_into().ok()?)
    }

    fn checked_shr(self, rhs: Self) -> Option<Self> {
        Self::checked_shr(self, rhs.try_into().ok()?)
    }
}

mod private {
    pub trait Sealed {}
    impl Sealed for u8 {}
    impl Sealed for u16 {}
    impl Sealed for u32 {}
    impl Sealed for u64 {}
    impl Sealed for u128 {}
    impl Sealed for usize {}
    impl Sealed for i16 {}
    impl Sealed for i32 {}
    impl Sealed for i64 {}
    impl Sealed for i128 {}
    impl Sealed for isize {}
    impl Sealed for f32 {}
    impl Sealed for f64 {}
}

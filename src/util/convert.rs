// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::convert::TryInto;

/// A trait to permit conversions into types using generics.
pub trait Conversions {
    /// A generic version of [std::convert::Into::into].
    ///
    /// This function has a slightly different name to avoid
    /// collisions with the Into trait (which is imported by default).
    fn into_type<T>(self) -> T
    where Self: Into<T> {
        Into::<T>::into(self)
    }
    /// A generic version of [std::convert::TryInto::try_into].
    ///
    /// Most of the type, the generics do not need to be specific because they
    /// can be assumed.
    fn try_into<T>(self) -> Result<T, Self::Error>
    where Self: TryInto<T> {
        TryInto::<T>::try_into(self)
    }
}
impl<T: ?Sized> Conversions for T {}

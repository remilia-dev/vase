// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.

/// A value representing how far back an error condition should unwind.
///
/// Generally, code should interact with [MayUnwind] unless its trying
/// to recover.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Unwind {
    /// The unwinding should proceed till it hits the outer 'block' (recovery point).
    Block,
    /// The unwinding is fatal and should go all the way up the function chain
    /// to the current compilation step.
    Fatal,
}
/// A result that represents a successful result or a requested unwinding.
///
/// Functions that don't plan on recovering on the unwind request, should
/// just use the `?` operator to return the unwind value if it occurs.
pub type MayUnwind<T> = Result<T, Unwind>;

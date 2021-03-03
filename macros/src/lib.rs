// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
#![feature(drain_filter)]

use proc_macro::TokenStream;

mod create_intos;
mod enum_with_properties;
mod util;

/// A macro to define an enum with specific properties.
///
/// The goal of this macro is to avoid having a stupid-long match
/// statement just to get a constant property.
///
/// The enum definition comes first and then an implementation that
/// contains at least one property method
///
/// Property methods are defined with a `#[property]` attribute.
///
/// The values for each enum are defined in a `#[values()]` attribute
/// in the order the property functions were defined. The value can
/// be almost any expression. The inner values of an enum are available
/// (tuple enum members are v# where # is the index of the member).
///
/// # Example
/// ```
/// # use vase_macros::enum_with_properties;
/// enum_with_properties! {
///     enum Region {
///         #[values("The Cold North", 65)]
///         North,
///         #[values("The Warm South", 32)]
///         South,
///     }
///     impl Region {
///         #[property]
///         fn name(&self) -> &'static str {}
///         #[property]
///         fn region_code(&self) -> u32 {}
///     }
/// }
///
/// #[test]
/// fn property() {
///     assert_eq!(Region::North.name(), "The Cold North");
///     assert_eq!(Region::South.region_code(), 32);
/// }
/// ```
#[proc_macro]
pub fn enum_with_properties(input: TokenStream) -> TokenStream {
    use enum_with_properties::EnumWithProperties;
    let enum_ = syn::parse_macro_input!(input as EnumWithProperties);
    enum_.to_stream()
}

/// A macro to create `impl From<Type>` blocks for enums containing a variant made of just a `Type`.
///
/// # Example
/// ```
/// # use vase_macros::create_intos;
/// #[create_intos]
/// enum MaybeResult<T, E> {
///     Some(T),
///     Err { error: E },
///     /// This one will not receive an into since it has no fields.
///     None,
///     /// This one will not receive an into since it has more than 1 field.
///     DoubleTrouble(T, E),
/// }
///
/// #[test]
/// fn into_conversion() {
///     let some: MaybeResult<u8, &'static str> = 0.into();
///     assert!(matches!(some, MaybeResult::Some(..)));
///     let err: MaybeResult<u8, &'static str> = "Test".into();
///     assert!(matches!(err, MaybeResult::Err(..)));
/// }
/// ```
#[proc_macro_attribute]
pub fn create_intos(_: TokenStream, item: TokenStream) -> TokenStream {
    let enum_ = syn::parse_macro_input!(item as syn::ItemEnum);
    create_intos::create_intos(enum_)
}

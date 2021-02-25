// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
#![feature(drain_filter)]

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{
        Parse,
        ParseStream,
    },
    spanned::Spanned,
    *,
};

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
    let enum_ = parse_macro_input!(input as EnumWithProperties);
    enum_.to_stream()
}

struct EnumWithProperties {
    enumeration: ItemEnum,
    implementation: ItemImpl,
    properties: Vec<ImplItemMethod>,
    enum_values: Vec<Vec<Expr>>,
}

impl EnumWithProperties {
    fn to_stream(self) -> proc_macro::TokenStream {
        let Self {
            enumeration,
            mut implementation,
            mut properties,
            enum_values,
        } = self;

        for (prop_index, property) in properties.iter_mut().enumerate() {
            let mut arms = Vec::new();
            for (enum_index, variant) in enumeration.variants.iter().enumerate() {
                let value = &enum_values[enum_index][prop_index];

                let variant_name = &variant.ident;
                match variant.fields {
                    Fields::Named(ref fields) => {
                        let fields: Vec<Ident> = (fields.named.iter()) //
                            .map(|f| f.ident.clone().unwrap()) //
                            .collect();
                        arms.push(quote! {
                            Self::#variant_name { #(#fields),* } => #value
                        })
                    },
                    Fields::Unnamed(ref fields) => {
                        let fields: Vec<Ident> = (0..fields.unnamed.len())
                            .map(|i| Ident::new(format!("v{}", i).as_str(), fields.span()))
                            .collect();
                        arms.push(quote! {
                            Self::#variant_name(#(#fields),*) => #value
                        })
                    },
                    Fields::Unit => {
                        arms.push(quote! {
                            Self::#variant_name => #value
                        });
                    },
                }
            }

            let mtch = quote! {
                {
                    #[allow(unused, clippy::pattern_type_mismatch)]
                    match self {
                        #(#arms,)*
                    }
                }
            }
            .into();

            let stmt = parse_macro_input!(mtch as Stmt);

            property.block.stmts.push(stmt);
        }

        for property in properties.into_iter() {
            implementation.items.push(ImplItem::Method(property));
        }

        proc_macro::TokenStream::from(quote! {
            #enumeration

            #implementation
        })
    }
}

impl Parse for EnumWithProperties {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut enumeration: ItemEnum = input.parse()?;
        let mut implementation: ItemImpl = input.parse()?;

        let properties_raw: Vec<ImplItem> = implementation
            .items
            .drain_filter(|item| {
                if let ImplItem::Method(method) = item {
                    return find_attribute("property", &mut method.attrs).is_some();
                }

                false
            })
            .collect();

        let properties: Vec<ImplItemMethod> = properties_raw
            .into_iter()
            .map(|item| {
                if let ImplItem::Method(method) = item {
                    method
                } else {
                    panic!("Only methods should have been added to the list.");
                }
            })
            .collect();

        if properties.is_empty() {
            return Err(Error::new(
                implementation.span(),
                "The impl block must contain at least one #[property] function.",
            ));
        }

        let mut enum_values = Vec::new();
        for variant in &mut enumeration.variants {
            let attribute = match find_attribute("values", &mut variant.attrs) {
                Some(attr) => attr,
                None => {
                    return Err(Error::new(
                        variant.span(),
                        "Variants must have a #[values()] attribute.",
                    ));
                },
            };
            let attribute_span = attribute.span();

            if properties.len() == 1 {
                let value: Expr = parse2(attribute.tokens)?;
                enum_values.push(vec![value]);
            } else {
                let attribute_tuple: ExprTuple = parse2(attribute.tokens)?;

                let values = attribute_tuple.elems;
                if values.len() != properties.len() {
                    return Err(Error::new(
                        attribute_span,
                        format!(
                            "Attribute has a different amount of values ({}) than have been declared ({})",
                            values.len(),
                            properties.len()
                        ),
                    ));
                }
                enum_values.push(values.into_iter().collect());
            }
        }

        Ok(EnumWithProperties {
            properties,
            implementation,
            enumeration,
            enum_values,
        })
    }
}

fn find_attribute(target: &str, attributes: &mut Vec<Attribute>) -> Option<Attribute> {
    for (i, attribute) in attributes.iter().enumerate() {
        if let Some(id) = attribute.path.get_ident() {
            if *id == target {
                return Some(attributes.remove(i));
            }
        }
    }

    None
}

// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
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
/// The functions defining the property come first and then the
/// definition of the enum. The property functions may only
/// take a version of self.
///
/// The values for each enum are defined in a `#[values()]` attribute
/// in the order the property functions were defined. The value can
/// be almost any expression. The inner values of an enum are available
/// (tuple enum members are v# where # is the index of the member).
///
/// # Example
/// ```
/// vase_macros::enum_with_properties! {
///     fn name(&self) -> &'static str {}
///     fn region_code(&self) -> u32 {}
///
///     enum Region {
///         #[values("The Cold North", 65)]
///         North,
///         #[values("The Warm South", 32)]
///         South,
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
    let EnumWithProperties {
        mut properties,
        enumeration,
        enum_values,
    } = parse_macro_input!(input as EnumWithProperties);

    let enum_name = enumeration.ident.clone();

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
                        #variant_name { #(#fields),* } => #value
                    })
                },
                Fields::Unnamed(ref fields) => {
                    let fields: Vec<Ident> = (0..fields.unnamed.len())
                        .map(|i| Ident::new(format!("v{}", i).as_str(), fields.span()))
                        .collect();
                    arms.push(quote! {
                        #variant_name(#(#fields),*) => #value
                    })
                },
                Fields::Unit => {
                    arms.push(quote! {
                        #variant_name => #value
                    });
                },
            }
        }

        let mtch = quote! {
            {
                use #enum_name::*;
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

    proc_macro::TokenStream::from(quote! {
        #enumeration

        impl #enum_name {
            #(#properties)*
        }
    })
}

struct EnumWithProperties {
    properties: Vec<ImplItemMethod>,
    enumeration: ItemEnum,
    enum_values: Vec<Vec<Expr>>,
}

impl Parse for EnumWithProperties {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut properties = Vec::new();
        loop {
            let fork = input.fork();
            if fork.parse::<Visibility>().is_ok() {
                if !fork.parse::<Token![fn]>().is_ok() {
                    break;
                }
            } else if !input.peek(Token![fn]) {
                break;
            }

            let method: ImplItemMethod = input.parse()?;
            let inputs = &method.sig.inputs;

            if inputs.len() != 1 {
                return Err(Error::new(
                    method.span(),
                    "Property functions may only take 1 argument.",
                ));
            } else if !matches!(inputs[0], FnArg::Receiver(..)) {
                return Err(Error::new(
                    method.span(),
                    "Property functions may only take a version of self.",
                ));
            } else {
                properties.push(method);
            }
        }

        if properties.is_empty() {
            return Err(Error::new(
                input.span(),
                "Properties are declared at the top as function signatures. You must declare at least one property.",
            ));
        }

        let mut enumeration: ItemEnum = input.parse()?;

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

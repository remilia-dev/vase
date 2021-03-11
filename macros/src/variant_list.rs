// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use proc_macro::TokenStream;
use quote::{
    quote,
    quote_spanned,
};
use syn::ItemEnum;

pub fn variant_list(enum_: ItemEnum) -> TokenStream {
    let mut variants = Vec::new();
    for variant in &enum_.variants {
        if variant.fields.len() != 0 {
            return quote_spanned! {
                variant.ident.span() =>
                compile_error!("variant_list cannot be used on enums that have variants with fields.");
            }.into();
        }
        let variant_name = &variant.ident;
        variants.push(quote! {
            Self::#variant_name
        })
    }

    let variant_count = variants.len();
    let enum_name = &enum_.ident;
    (quote! {
        #enum_

        impl #enum_name {
            pub const VARIANTS: [Self; #variant_count] = [#(#variants,)*];
        }
    })
    .into()
}

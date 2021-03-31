// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use quote::quote;
use syn::ItemEnum;

pub fn variant_names(enum_: ItemEnum) -> proc_macro::TokenStream {
    let mut match_arms = Vec::new();
    for variant in &enum_.variants {
        let variant_name = &variant.ident;
        let variant_name_str = variant_name.to_string();
        match_arms.push(quote! {
            Self::#variant_name { .. } => #variant_name_str,
        })
    }

    let enum_name = &enum_.ident;
    (quote! {
        #enum_

        impl #enum_name {
            pub fn variant_name(&self) -> &'static str {
                match *self {
                    #(#match_arms)*
                }
            }
        }
    })
    .into()
}

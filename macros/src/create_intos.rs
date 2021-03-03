// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use quote::{
    quote,
    ToTokens,
};
use syn::{
    parse2,
    Fields,
    GenericParam,
    ItemEnum,
    ItemImpl,
    Variant,
};

pub fn create_intos(enum_: ItemEnum) -> proc_macro::TokenStream {
    let mut into_impls = Vec::new();
    for variant in &enum_.variants {
        if variant.fields.len() != 1 {
            continue;
        }

        match gen_variant(&enum_, &variant) {
            Some(Ok(var_impl)) => into_impls.push(var_impl),
            Some(Err(err)) => return err.into_compile_error().into(),
            None => {},
        }
    }

    proc_macro::TokenStream::from(quote! {
        #enum_

        #(#into_impls)*
    })
}

fn gen_variant(enum_: &ItemEnum, variant: &Variant) -> Option<syn::Result<ItemImpl>> {
    let enum_name = &enum_.ident;
    let generics = &enum_.generics.params;
    let where_clause = &enum_.generics.where_clause;

    let variant_name = &variant.ident;

    let field = match variant.fields {
        Fields::Named(ref fields) => &fields.named[0],
        Fields::Unnamed(ref fields) => &fields.unnamed[0],
        _ => return None,
    };
    let field_type = &field.ty;

    let creation = match field.ident {
        Some(ref name) => quote! { Self::#variant_name { #name: v } },
        None => quote! { Self::#variant_name(v) },
    };

    let mut generic_values = Vec::new();
    for generic in generics {
        match generic {
            GenericParam::Const(ref con) => {
                generic_values.push(con.ident.to_token_stream());
            },
            GenericParam::Lifetime(ref lifetime) => {
                generic_values.push(lifetime.lifetime.to_token_stream());
            },
            GenericParam::Type(ref typ) => {
                generic_values.push(typ.ident.to_token_stream());
            },
        }
    }

    let stream = quote! {
        impl#generics From<#field_type> for #enum_name<#(#generic_values,)*>
        #where_clause
        {
            fn from(v: #field_type) -> Self {
                #creation
            }
        }
    };

    Some(parse2(stream))
}

//! `Serialize` and `Deserialize` for every struct and enum in an api module.
//!
//! The route file is included unchanged, so an error in that file still points
//! at the file. A proxy type derives serde, which keeps the JSON shape of a
//! hand-written derive, including newtype structs and externally tagged enums.
//! A type that already derives or implements one side is left alone on that side.

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::ext::IdentExt;
use syn::punctuated::Punctuated;

const SERDE: &str = "::gasket_ui::serde";

pub(crate) fn impls(source: &str) -> TokenStream2 {
    let Ok(file) = syn::parse_file(source) else {
        return quote! {};
    };
    let (manual_ser, manual_de) = manual_impls(&file);
    let mut out = TokenStream2::new();
    for item in &file.items {
        match item {
            syn::Item::Struct(item) => emit(
                &mut out,
                &item.ident,
                &item.generics,
                &item.attrs,
                Shape::Struct(&item.fields),
                &manual_ser,
                &manual_de,
            ),
            syn::Item::Enum(item) => emit(
                &mut out,
                &item.ident,
                &item.generics,
                &item.attrs,
                Shape::Enum(&item.variants),
                &manual_ser,
                &manual_de,
            ),
            _ => {}
        }
    }
    out
}

#[derive(Clone, Copy)]
enum Shape<'a> {
    Struct(&'a syn::Fields),
    Enum(&'a Punctuated<syn::Variant, syn::token::Comma>),
}

fn emit(
    out: &mut TokenStream2,
    ident: &syn::Ident,
    generics: &syn::Generics,
    attrs: &[syn::Attribute],
    shape: Shape<'_>,
    manual_ser: &HashSet<String>,
    manual_de: &HashSet<String>,
) {
    let name = ident.to_string();
    let cfg = cfgs(attrs);
    if !manual_ser.contains(&name) && !has_derive(attrs, "Serialize") {
        out.extend(one_side(ident, generics, &cfg, shape, Side::Serialize));
    }
    if !manual_de.contains(&name) && !has_derive(attrs, "Deserialize") {
        out.extend(one_side(ident, generics, &cfg, shape, Side::Deserialize));
    }
}

enum Side {
    Serialize,
    Deserialize,
}

fn one_side(
    ident: &syn::Ident,
    generics: &syn::Generics,
    cfg: &[&syn::Attribute],
    shape: Shape<'_>,
    side: Side,
) -> TokenStream2 {
    let as_ref = matches!(side, Side::Serialize) && has_field(shape);
    let proxy = proxy_ident(ident, &side);
    let proxy_generics = proxy_generics(generics, as_ref);
    let body = match shape {
        Shape::Struct(fields) => struct_body(fields, as_ref),
        Shape::Enum(variants) => enum_body(variants, as_ref),
    };
    let derive = match side {
        Side::Serialize => quote! { ::gasket_ui::serde::Serialize },
        Side::Deserialize => quote! { ::gasket_ui::serde::Deserialize },
    };
    let keyword = match shape {
        Shape::Struct(_) => quote! { struct },
        Shape::Enum(_) => quote! { enum },
    };
    let impl_generics = impl_generics(generics, matches!(side, Side::Deserialize));
    // `'de` lives on the impl, not on the user's type.
    let (impl_g, _, where_g) = impl_generics.split_for_impl();
    let ty_g = type_generics(generics);
    let apply = match side {
        Side::Serialize => serialize_body(generics, &proxy, shape),
        Side::Deserialize => deserialize_body(generics, &proxy, shape),
    };
    let trait_path = match side {
        Side::Serialize => quote! { ::gasket_ui::serde::Serialize },
        Side::Deserialize => quote! { ::gasket_ui::serde::Deserialize<'de> },
    };
    quote! {
        #(#cfg)*
        #[allow(non_camel_case_types)]
        #[derive(#derive)]
        #[serde(crate = #SERDE)]
        #keyword #proxy #proxy_generics #body

        #(#cfg)*
        #[automatically_derived]
        impl #impl_g #trait_path for #ident #ty_g #where_g {
            #apply
        }
    }
}

fn serialize_body(generics: &syn::Generics, proxy: &syn::Ident, shape: Shape<'_>) -> TokenStream2 {
    let as_ref = has_field(shape);
    let ty = proxy_type(proxy, generics, as_ref);
    let value = match shape {
        Shape::Struct(fields) => struct_ser_value(proxy, generics, fields),
        Shape::Enum(variants) => enum_ser_value(proxy, generics, variants),
    };
    quote! {
        fn serialize<__S>(&self, serializer: __S) -> Result<__S::Ok, __S::Error>
        where
            __S: ::gasket_ui::serde::Serializer,
        {
            let __gasket: #ty = #value;
            ::gasket_ui::serde::Serialize::serialize(&__gasket, serializer)
        }
    }
}

fn deserialize_body(
    generics: &syn::Generics,
    proxy: &syn::Ident,
    shape: Shape<'_>,
) -> TokenStream2 {
    let ty = type_generics(generics);
    let build = match shape {
        Shape::Struct(fields) => struct_de_build(fields),
        Shape::Enum(variants) => enum_de_build(proxy, variants),
    };
    quote! {
        fn deserialize<__D>(deserializer: __D) -> Result<Self, __D::Error>
        where
            __D: ::gasket_ui::serde::Deserializer<'de>,
        {
            let __gasket: #proxy #ty = #proxy::deserialize(deserializer)?;
            #build
        }
    }
}

fn struct_body(fields: &syn::Fields, as_ref: bool) -> TokenStream2 {
    match fields {
        syn::Fields::Named(named) => {
            let fields = named.named.iter().map(|field| {
                let cfg = cfgs(&field.attrs);
                let name = &field.ident;
                let ty = ref_ty(&field.ty, as_ref);
                quote! { #(#cfg)* #name: #ty }
            });
            quote! { { #(#fields),* } }
        }
        syn::Fields::Unnamed(unnamed) => {
            let fields = unnamed.unnamed.iter().map(|field| {
                let cfg = cfgs(&field.attrs);
                let ty = ref_ty(&field.ty, as_ref);
                quote! { #(#cfg)* #ty }
            });
            quote! { (#(#fields),*); }
        }
        syn::Fields::Unit => quote! { ; },
    }
}

fn enum_body(variants: &Punctuated<syn::Variant, syn::token::Comma>, as_ref: bool) -> TokenStream2 {
    let variants = variants.iter().map(|variant| {
        let cfg = cfgs(&variant.attrs);
        let ident = &variant.ident;
        let fields = match &variant.fields {
            syn::Fields::Named(named) => {
                let fields = named.named.iter().map(|field| {
                    let cfg = cfgs(&field.attrs);
                    let name = &field.ident;
                    let ty = ref_ty(&field.ty, as_ref);
                    quote! { #(#cfg)* #name: #ty }
                });
                quote! { { #(#fields),* } }
            }
            syn::Fields::Unnamed(unnamed) => {
                let fields = unnamed.unnamed.iter().map(|field| {
                    let cfg = cfgs(&field.attrs);
                    let ty = ref_ty(&field.ty, as_ref);
                    quote! { #(#cfg)* #ty }
                });
                quote! { (#(#fields),*) }
            }
            syn::Fields::Unit => quote! {},
        };
        quote! { #(#cfg)* #ident #fields }
    });
    quote! { { #(#variants),* } }
}

fn struct_ser_value(
    proxy: &syn::Ident,
    generics: &syn::Generics,
    fields: &syn::Fields,
) -> TokenStream2 {
    let path = proxy_expr(proxy, generics, has_fields(fields));
    match fields {
        syn::Fields::Named(named) => {
            let fields = named.named.iter().map(|field| {
                let cfg = cfgs(&field.attrs);
                let name = &field.ident;
                quote! { #(#cfg)* #name: &self.#name }
            });
            quote! { #path { #(#fields),* } }
        }
        syn::Fields::Unnamed(unnamed) => {
            let fields = unnamed.unnamed.iter().enumerate().map(|(index, field)| {
                let cfg = cfgs(&field.attrs);
                let index = syn::Index::from(index);
                quote! { #(#cfg)* &self.#index }
            });
            quote! { #path(#(#fields),*) }
        }
        syn::Fields::Unit => path,
    }
}

fn enum_ser_value(
    proxy: &syn::Ident,
    generics: &syn::Generics,
    variants: &Punctuated<syn::Variant, syn::token::Comma>,
) -> TokenStream2 {
    if variants.is_empty() {
        return quote! { match self {} };
    }
    let path = proxy_expr(proxy, generics, has_variant_field(variants));
    let arms = variants.iter().map(|variant| {
        let cfg = cfgs(&variant.attrs);
        let ident = &variant.ident;
        match &variant.fields {
            syn::Fields::Named(named) => {
                let names: Vec<_> = named.named.iter().map(|field| &field.ident).collect();
                quote! {
                    #(#cfg)*
                    Self::#ident { #(#names),* } => #path::#ident { #(#names),* }
                }
            }
            syn::Fields::Unnamed(unnamed) => {
                let binds: Vec<_> = (0..unnamed.unnamed.len())
                    .map(|index| format_ident!("__f{index}"))
                    .collect();
                quote! {
                    #(#cfg)*
                    Self::#ident(#(#binds),*) => #path::#ident(#(#binds),*)
                }
            }
            syn::Fields::Unit => quote! {
                #(#cfg)*
                Self::#ident => #path::#ident
            },
        }
    });
    quote! { match self { #(#arms),* } }
}

fn struct_de_build(fields: &syn::Fields) -> TokenStream2 {
    match fields {
        syn::Fields::Named(named) if named.named.is_empty() => quote! {
            let _ = __gasket;
            Ok(Self {})
        },
        syn::Fields::Named(named) => {
            let fields = named.named.iter().map(|field| {
                let cfg = cfgs(&field.attrs);
                let name = &field.ident;
                quote! { #(#cfg)* #name: __gasket.#name }
            });
            quote! { Ok(Self { #(#fields),* }) }
        }
        syn::Fields::Unnamed(unnamed) if unnamed.unnamed.is_empty() => quote! {
            let _ = __gasket;
            Ok(Self())
        },
        syn::Fields::Unnamed(unnamed) => {
            let fields = unnamed.unnamed.iter().enumerate().map(|(index, field)| {
                let cfg = cfgs(&field.attrs);
                let index = syn::Index::from(index);
                quote! { #(#cfg)* __gasket.#index }
            });
            quote! { Ok(Self(#(#fields),*)) }
        }
        syn::Fields::Unit => quote! {
            let _ = __gasket;
            Ok(Self)
        },
    }
}

fn enum_de_build(
    proxy: &syn::Ident,
    variants: &Punctuated<syn::Variant, syn::token::Comma>,
) -> TokenStream2 {
    if variants.is_empty() {
        return quote! { Ok(match __gasket {}) };
    }
    let arms = variants.iter().map(|variant| {
        let cfg = cfgs(&variant.attrs);
        let ident = &variant.ident;
        match &variant.fields {
            syn::Fields::Named(named) => {
                let names: Vec<_> = named.named.iter().map(|field| &field.ident).collect();
                quote! {
                    #(#cfg)*
                    #proxy::#ident { #(#names),* } => Self::#ident { #(#names),* }
                }
            }
            syn::Fields::Unnamed(unnamed) => {
                let binds: Vec<_> = (0..unnamed.unnamed.len())
                    .map(|index| format_ident!("__f{index}"))
                    .collect();
                quote! {
                    #(#cfg)*
                    #proxy::#ident(#(#binds),*) => Self::#ident(#(#binds),*)
                }
            }
            syn::Fields::Unit => quote! {
                #(#cfg)*
                #proxy::#ident => Self::#ident
            },
        }
    });
    quote! { Ok(match __gasket { #(#arms),* }) }
}

fn ref_ty(ty: &syn::Type, as_ref: bool) -> TokenStream2 {
    if as_ref {
        quote! { &'__gasket #ty }
    } else {
        quote! { #ty }
    }
}

fn has_field(shape: Shape<'_>) -> bool {
    match shape {
        Shape::Struct(fields) => has_fields(fields),
        Shape::Enum(variants) => has_variant_field(variants),
    }
}

fn has_fields(fields: &syn::Fields) -> bool {
    fields.iter().next().is_some()
}

fn has_variant_field(variants: &Punctuated<syn::Variant, syn::token::Comma>) -> bool {
    variants.iter().any(|variant| has_fields(&variant.fields))
}

fn proxy_ident(ident: &syn::Ident, side: &Side) -> syn::Ident {
    let kind = match side {
        Side::Serialize => "Ser",
        Side::Deserialize => "De",
    };
    format_ident!("__Gasket{kind}{}", ident.unraw())
}

fn proxy_generics(generics: &syn::Generics, as_ref: bool) -> syn::Generics {
    let mut generics = generics.clone();
    strip_defaults(&mut generics);
    if as_ref {
        generics.params.insert(0, syn::parse_quote!('__gasket));
    }
    generics
}

fn impl_generics(generics: &syn::Generics, deserialize: bool) -> syn::Generics {
    let mut generics = generics.clone();
    strip_defaults(&mut generics);
    if deserialize {
        let lifetimes: Vec<_> = generics
            .lifetimes()
            .map(|param| param.lifetime.clone())
            .collect();
        generics.params.insert(0, syn::parse_quote!('de));
        for param in &mut generics.params {
            if let syn::GenericParam::Type(param) = param {
                param
                    .bounds
                    .push(syn::parse_quote!(::gasket_ui::serde::de::Deserialize<'de>));
            }
        }
        for lifetime in lifetimes {
            generics
                .make_where_clause()
                .predicates
                .push(syn::parse_quote!('de: #lifetime));
        }
    } else {
        for param in &mut generics.params {
            if let syn::GenericParam::Type(param) = param {
                param
                    .bounds
                    .push(syn::parse_quote!(::gasket_ui::serde::Serialize));
            }
        }
    }
    generics
}

fn type_generics(generics: &syn::Generics) -> TokenStream2 {
    let mut generics = generics.clone();
    strip_defaults(&mut generics);
    let (_, ty, _) = generics.split_for_impl();
    quote! { #ty }
}

/// Type position: `Proxy<'_>` or `Proxy<T>`.
fn proxy_type(proxy: &syn::Ident, generics: &syn::Generics, as_ref: bool) -> TokenStream2 {
    let args = type_args(generics);
    if as_ref {
        quote! { #proxy <'_ #(, #args)*> }
    } else if args.is_empty() {
        quote! { #proxy }
    } else {
        quote! { #proxy <#(#args),*> }
    }
}

/// Expression position: `Proxy::<'_>` or `Proxy::<T>`.
fn proxy_expr(proxy: &syn::Ident, generics: &syn::Generics, as_ref: bool) -> TokenStream2 {
    let args = type_args(generics);
    if as_ref {
        quote! { #proxy :: <'_ #(, #args)*> }
    } else if args.is_empty() {
        quote! { #proxy }
    } else {
        quote! { #proxy :: <#(#args),*> }
    }
}

fn type_args(generics: &syn::Generics) -> Vec<TokenStream2> {
    generics
        .params
        .iter()
        .map(|param| match param {
            syn::GenericParam::Lifetime(param) => {
                let lifetime = &param.lifetime;
                quote! { #lifetime }
            }
            syn::GenericParam::Type(param) => {
                let ident = &param.ident;
                quote! { #ident }
            }
            syn::GenericParam::Const(param) => {
                let ident = &param.ident;
                quote! { #ident }
            }
        })
        .collect()
}

fn strip_defaults(generics: &mut syn::Generics) {
    for param in &mut generics.params {
        match param {
            syn::GenericParam::Type(param) => param.default = None,
            syn::GenericParam::Const(param) => param.default = None,
            syn::GenericParam::Lifetime(_) => {}
        }
    }
}

fn cfgs(attrs: &[syn::Attribute]) -> Vec<&syn::Attribute> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("cfg"))
        .collect()
}

fn has_derive(attrs: &[syn::Attribute], trait_name: &str) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("derive") {
            return false;
        }
        let Ok(paths) =
            attr.parse_args_with(Punctuated::<syn::Path, syn::Token![,]>::parse_terminated)
        else {
            return false;
        };
        paths.iter().any(|path| {
            path.segments
                .last()
                .is_some_and(|seg| seg.ident == trait_name)
        })
    })
}

fn manual_impls(file: &syn::File) -> (HashSet<String>, HashSet<String>) {
    let mut ser = HashSet::new();
    let mut de = HashSet::new();
    for item in &file.items {
        let syn::Item::Impl(item) = item else {
            continue;
        };
        let Some((_, path, _)) = &item.trait_ else {
            continue;
        };
        let Some(trait_name) = path.segments.last() else {
            continue;
        };
        let syn::Type::Path(self_ty) = item.self_ty.as_ref() else {
            continue;
        };
        if self_ty.qself.is_some() {
            continue;
        }
        let Some(self_name) = self_ty.path.segments.last() else {
            continue;
        };
        let bucket = if trait_name.ident == "Serialize" {
            &mut ser
        } else if trait_name.ident == "Deserialize" {
            &mut de
        } else {
            continue;
        };
        bucket.insert(self_name.ident.to_string());
    }
    (ser, de)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_struct_gets_both_traits() {
        let tokens = impls("pub struct Response { pub message: String }").to_string();
        assert!(tokens.contains("Serialize for Response"));
        assert!(tokens.contains("Deserialize < 'de > for Response"));
        assert!(tokens.contains("message"));
    }

    #[test]
    fn existing_derive_is_left_alone() {
        let tokens = impls(
            "#[derive(serde::Serialize, serde::Deserialize)] pub struct Response { pub message: String }",
        )
        .to_string();
        assert!(tokens.is_empty());
    }

    #[test]
    fn manual_serialize_still_gets_deserialize() {
        let tokens = impls(
            "pub struct Response { pub message: String } \
             impl serde::Serialize for Response { \
                 fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> { unimplemented!() } \
             }",
        )
        .to_string();
        assert!(!tokens.contains("Serialize for Response"));
        assert!(tokens.contains("Deserialize < 'de > for Response"));
    }

    #[test]
    fn cfg_on_the_type_is_copied() {
        let tokens = impls("#[cfg(test)] struct Echo(u8);").to_string();
        assert!(tokens.contains("cfg (test)"));
        assert!(tokens.contains("for Echo"));
    }
}

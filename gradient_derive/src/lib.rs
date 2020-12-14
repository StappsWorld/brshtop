use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

const PREFIXES: [&str; 9] = [
    "temp",
    "cpu",
    "free",
    "cached",
    "available",
    "used",
    "download",
    "upload",
    "process",
];
const SUFFIXES: [&str; 3] = ["start", "mid", "end"];

fn get_items(item: &syn::DeriveInput) -> Vec<proc_macro2::Ident> {
    let mut items = vec![];
    if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { named, .. }),
        ..
    }) = item.data.to_owned()
    {
        let mut iter = named.into_iter();
        while let Some(syn::Field {
            ident: Some(ident), ..
        }) = iter.next()
        {
            items.push(ident);
        }
    }

    items
}

fn get_defaults(item: &syn::DeriveInput) -> Vec<(proc_macro2::Ident, Option<String>)> {
    match item.data.clone() {
        syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Named(syn::FieldsNamed { named, .. }),
            ..
        }) => named
            .into_iter()
            .map(|field| {
                (
                    field.ident.unwrap(),
                    field.attrs.get(0).map(attr_str).flatten(),
                )
            })
            .collect(),
        _ => unreachable!(),
    }
}

fn attr_ident(attr: &syn::Attribute) -> Option<&syn::Ident> {
    let syn::Path { segments, .. } = &attr.path;
    if let Some(syn::PathSegment { ident, .. }) = segments.iter().next() {
        Some(ident)
    } else {
        None
    }
}
fn attr_str(attr: &syn::Attribute) -> Option<String> {
    format!(
        "{}",
        syn::parse::<TokenStream2>(TokenStream::from(attr.tokens.clone())).unwrap()
    )
    .split('"')
    .nth(1)
    .map(<_ as ToString>::to_string)
}
fn attr_lit(attr: &syn::Attribute) -> Option<proc_macro2::Literal> {
    match attr.tokens.clone().into_iter().nth(1) {
        Some(proc_macro2::TokenTree::Literal(lit)) => Some(lit),
        _ => None,
    }
}

fn literal_to_ident(lit: proc_macro2::Literal) -> syn::Ident {
    let mut s = format!("{}", lit);
    s.pop();
    s.remove(0);
    syn::Ident::new(&s, proc_macro2::Span::call_site())
}

fn get_value_ty(item: &syn::DeriveInput) -> Option<syn::Ident> {
    item.attrs
        .iter()
        .map(|attr| (attr_ident(attr).unwrap(), attr_lit(attr).unwrap()))
        .find(|(ident, _)| ident == &&syn::Ident::new("value_type", proc_macro2::Span::call_site()))
        .map(|optional| literal_to_ident(optional.1))
}

fn gradient_fields() -> Vec<String> {
    PREFIXES
        .iter()
        .map(|prefix| {
            SUFFIXES.iter().fold(vec![], |mut acc, suffix| {
                acc.push(format!("{}_{}", prefix, suffix));
                acc
            })
        })
        .flatten()
        .collect()
}

fn has_all_gradient_fields(s: &syn::Data) -> Result<(), Vec<String>> {
    let field_names: Vec<String> = match s {
        syn::Data::Struct(ds) => {
            match ds.fields.clone() {
                syn::Fields::Named(syn::FieldsNamed { named: fields, .. }) => fields
                    .iter()
                    // Filter shouldn't lose any, because named exists, but what can ya do :^)
                    .filter_map(|field| Some(field.ident.clone()?.to_string()))
                    .collect(),
                _ => {
                    panic!("Unnamed fields are not supported >:^(")
                }
            }
        }
        _ => panic!("Don't be an ass, Jake"),
    };

    let missing: Vec<String> = gradient_fields()
        .iter()
        .filter(|field_name| !field_names.contains(field_name))
        .map(<_ as ToString>::to_string)
        .collect();

    if missing.is_empty() {
        Ok(())
    } else {
        Err(missing)
    }
}

#[proc_macro_derive(Gradient)]
pub fn derive_gradient(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let items = get_items(&input);
    match has_all_gradient_fields(&input.data) {
        Ok(_) => {}
        Err(missing) => {
            panic!(format!("Missing fields: {:#?}", missing))
        }
    }
    let struct_ident = &input.ident;

    let gen = quote! {
        impl ::gradient::Gradient for #struct_ident {
            fn gradient(
                &self,
            ) -> ::std::collections::HashMap<
                ::std::string::String,
                (
                    ::std::string::String,
                    ::std::string::String,
                    ::std::string::String,
                ),
            > {
                let mut gradient_map = ::std::collections::HashMap::new();


                gradient_map
            }
        }
    };
    gen.into()
}

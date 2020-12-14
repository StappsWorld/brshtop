use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

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

#[proc_macro_derive(FromMap, attributes(value_type))]
pub fn derive_from_map(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let items = get_items(&input);
    let value_type = get_value_ty(&input);
    let struct_ident = &input.ident;

    let body = items
        .iter()
        .map(|ident| {
            quote! {
                #ident: map.get(stringify!(#ident).into()).cloned().unwrap_or_default(),
            }
        })
        .fold(TokenStream2::new(), |mut acc, cur| {
            acc.extend(cur.into_iter());
            acc
        });

    let gen = quote! {
        impl ::from_map::FromMap for #struct_ident {
            type Value = #value_type;

            fn from_map(map: HashMap<String, Self::Value>) -> Self {
                Self {
                    #body
                }
            }
        }
    };
    gen.into()
}

#[proc_macro_derive(FromMapDefault, attributes(value_type, default))]
pub fn derive_from_map_default(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let items = get_items(&input);
    let value_type = get_value_ty(&input);
    let struct_ident = &input.ident;
    let defaults = get_defaults(&input);

    let default_map = defaults
        .iter()
        .fold(TokenStream2::new(), |mut acc, (ident, default)| {
            let default = default.clone().unwrap_or_default();
            acc.extend(
                quote! {
                    (stringify!(#ident), #default),
                }
                .into_iter(),
            );
            acc
        });
    let body = items
        .iter()
        .map(|ident| {
            quote! {
                #ident: map.get(stringify!(#ident).into()).cloned().unwrap_or(default_map.get(stringify!(#ident).into()).cloned().unwrap()),
            }
        })
        .fold(TokenStream2::new(), |mut acc, cur| {
            acc.extend(cur.into_iter());
            acc
        });

    let gen = quote! {
        impl ::from_map::FromMapDefault for #struct_ident {
            type Value = #value_type;

            fn from_map_default(map: HashMap<String, Self::Value>) -> Self {
                let default_map = Self::default_map();
                Self {
                    #body
                }
            }
            fn default_map() -> HashMap<String, <Self as FromMapDefault>::Value> {
                let map = vec![
                    #default_map
                ];

                map
                    .iter()
                    .map(|(field, colstr)| (field.to_string(), Color::new(colstr).unwrap()))
                    .collect()

            }
        }
        impl Default for #struct_ident {
            fn default() -> Self {
                Self::from_map_default(HashMap::new())
            }
        }
    };
    gen.into()
}

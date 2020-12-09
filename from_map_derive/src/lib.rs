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

fn attr_ident(attr: &syn::Attribute) -> Option<&syn::Ident> {
    let syn::Path { segments, .. } = &attr.path;
    if let Some(syn::PathSegment { ident, .. }) = segments.iter().next() {
        Some(ident)
    } else {
        None
    }
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
pub fn derive_into_query(input: TokenStream) -> TokenStream {
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
    println!("{:#?}", gen);
    gen.into()
}

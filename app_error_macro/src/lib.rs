#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parenthesized, parse_macro_input, token, Ident, Path, Token, Type, Visibility};

struct MacroInput {
    vis: Visibility,
    enum_token: Token![enum],
    name: Ident,
    brace_token: token::Brace,
    entries: Punctuated<Entry, Token![,]>,
}

enum Entry {
    UseVariants(Path),
    Variant { name: Ident, ty: Type },
}

impl Parse for MacroInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let vis: Visibility = input.parse()?;
        let enum_token: Token![enum] = input.parse()?;
        let name: Ident = input.parse()?;
        let content;
        let brace_token = syn::braced!(content in input);
        let entries = content.parse_terminated(Entry::parse, Token![,])?;

        Ok(Self {
            vis,
            enum_token,
            name,
            brace_token,
            entries,
        })
    }
}

impl Parse for Entry {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        if input.peek(Ident) {
            let fork = input.fork();
            let ident: Ident = fork.parse()?;
            if ident == "use_variants" {
                let _: Ident = input.parse()?;
                let content;
                parenthesized!(content in input);
                let path: Path = content.parse()?;
                return Ok(Self::UseVariants(path));
            }
        }

        let name: Ident = input.parse()?;
        let content;
        parenthesized!(content in input);
        let ty: Type = content.parse()?;
        Ok(Self::Variant { name, ty })
    }
}

#[proc_macro]
pub fn flat_error_enum(input: TokenStream) -> TokenStream {
    let MacroInput {
        vis,
        enum_token,
        name,
        brace_token,
        entries,
    } = parse_macro_input!(input as MacroInput);

    let _ = enum_token;
    let _ = brace_token;

    let mut flattened = Vec::new();

    for entry in entries {
        match entry {
            Entry::UseVariants(path) => {
                flattened.push(quote! { #path!() });
            }
            Entry::Variant { name, ty } => {
                flattened.push(quote! { #name(#ty) });
            }
        }
    }

    let expanded = quote! {
        crate::__flat_error_enum_impl! {
            #vis enum #name {
                #(#flattened,)*
            }
        }
    };

    expanded.into()
}

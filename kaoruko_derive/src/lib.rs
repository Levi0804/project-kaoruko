use proc_macro::TokenStream;
use proc_macro2::{Literal, TokenTree};
use quote::{ToTokens, quote};
use std::collections::HashMap;
use syn::{Data, DataEnum, DeriveInput, Meta};
mod types;

use types::*;

// NEXT: parse the queries.
#[proc_macro_derive(CommandParser, attributes(config))]
pub fn derive(input: TokenStream) -> TokenStream {
    match derive_internal(input) {
        Ok(stream) => stream.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn derive_internal(input: TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let input = syn::parse2::<DeriveInput>(input.into())?;
    let name = &input.ident;

    let Data::Enum(enumeration) = &input.data else {
        panic!("macro can only be applied on enums");
    };

    // we don't care if enum has no variants
    if enumeration.variants.is_empty() {
        return Ok(quote! {});
    }

    for variant in &enumeration.variants {
        for attr in &variant.attrs {
            let Meta::List(list) = &attr.meta else {
                continue;
            };
            // for token in list.tokens.clone().into_iter() {
            //     token
            //         .to_string()
            //         .parse::<SupportedArguments>()
            //         .map_err(|_| new_error(&token, "unexpected tken"))?;
            // }
            check_duplicates(list.tokens.clone())?;
            check_required(
                list.tokens.clone(),
                &["description", "roles"],
                &variant.ident,
            )?;
        }
    }

    let aliases = parse_aliases(enumeration)?;
    let descriptions = parse_description(enumeration)?;
    let roles = parse_roles(enumeration)?;

    let condition_blocks = roles.iter().map(|(variant, roles)| {
        let condition = roles.iter().map(|literal| {
            // TODO: improve this
            if &literal.to_string() == "\"developer\"" {
                quote! {
                    let mut only_developer = false;
                    for role in &roles {
                        if role == #literal {
                            only_developer = true;
                        }
                    }
                    let auth_default = crate::Auth::default();
                    let discord_id = &auth.as_ref().unwrap_or(&&auth_default).id;
                    if !only_developer && discord_id != "988839581384323083" {
                        return Err(anyhow::anyhow!("you are not eligible to use this command"));
                    }
                }
            } else if &literal.to_string() == "\"creator\"" {
                quote! {
                    let mut only_creator = false;
                    for role in &roles {
                        if role == #literal {
                            only_creator = true;
                        }
                    }
                    let auth_default = crate::Auth::default();
                    let discord_id = &auth.as_ref().unwrap_or(&&auth_default).id;
                    if discord_id == "988839581384323083" { return Ok(#name::#variant); }
                    if !only_creator && &room_creator != discord_id {
                        return Err(anyhow::anyhow!("you are not eligible to use this command"));
                    }
                }
            } else {
                // covers "\"anyone\""
                quote! {}
            }
        });
        quote! {
            #name::#variant => {
                #(#condition)*
                return Ok(#name::#variant);
            }
        }
    });

    let stream = quote! {
        impl std::str::FromStr for #name {
            type Err = anyhow::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                   #(#aliases,)*
                    _ => Err(anyhow::anyhow!("unknown command")),
                }
            }
        }

        impl #name {
            pub fn help(&self) -> &'static str {
                match self {
                   #(#descriptions,)*
                }
            }
        }

        pub trait CommandParserTrait {
            fn parse_command(
                &self,
                roles: Vec<String>,
                auth: Option<&crate::Auth>,
                room_creator: String
            ) -> anyhow::Result<#name>;
        }

        impl CommandParserTrait for &str {
            fn parse_command(
                &self,
                roles: Vec<String>,
                auth: Option<&crate::Auth>,
                room_creator: String
            ) -> anyhow::Result<#name> {
                match self.parse::<#name>() {
                    Ok(cmd) => match cmd {
                        #(#condition_blocks)*
                    },
                    Err(err) => Err(err),
                }
            }
        }
    };

    Ok(stream)
}

fn new_error<T: ToTokens>(token_stream: &T, error: &str) -> syn::Error {
    syn::Error::new_spanned(token_stream, error)
}

fn check_duplicates(stream: proc_macro2::TokenStream) -> Result<(), syn::Error> {
    let mut idents = Vec::<syn::Ident>::new();
    for token in stream {
        if let TokenTree::Ident(ref ident) = token {
            if idents.contains(ident) {
                return Err(new_error(
                    ident,
                    &format!("duplicate attribute found: {ident}"),
                ));
            }
            idents.push(ident.clone());
        }
    }
    Ok(())
}

fn check_required(
    stream: proc_macro2::TokenStream,
    attrs: &[&'static str],
    variant: &syn::Ident,
) -> Result<(), syn::Error> {
    let mut contains = false;
    for attr in attrs {
        for token in stream.clone() {
            if let TokenTree::Ident(ref ident) = token {
                if attr == &ident.to_string() {
                    contains = true;
                }
            }
        }
        if !contains {
            return Err(new_error(
                &variant,
                &format!("variant `{variant}` must define `{attr}` attribute"),
            ));
        }
        contains = false;
    }
    Ok(())
}

// TODO: better error handling
fn parse_description(enumeration: &DataEnum) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let mut descriptions = Vec::<proc_macro2::TokenStream>::new();
    for variant in &enumeration.variants {
        for attr in &variant.attrs {
            let Meta::List(ref list) = attr.meta else {
                continue;
            };
            let mut iter = list.tokens.clone().into_iter();
            while let Some(token) = iter.next() {
                if let TokenTree::Ident(ref ident) = token {
                    let SupportedArguments::Alias = ident
                        .to_string()
                        .parse::<SupportedArguments>()
                        .map_err(|_| {
                            new_error(ident, &format!("unknown variant attribute `{ident}"))
                        })?
                    else {
                        continue;
                    };
                    let Some(TokenTree::Punct(ref punct)) = iter.next() else {
                        return Err(new_error(
                            &ident,
                            &format!("expected punct `=` after `{ident}`"),
                        ));
                    };
                    if punct.to_string() != "=" {
                        return Err(new_error(
                            &punct,
                            &format!("expected punct `=` after `{ident}`"),
                        ));
                    }
                }
                // move this inside the last curly with proper parsing
                if let Some(TokenTree::Literal(ref literal)) = iter.next() {
                    let variant = &variant.ident;
                    descriptions.push(quote! { Self::#variant => #literal });
                }
            }
        }
    }
    Ok(descriptions)
}

fn parse_aliases(enumeration: &DataEnum) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let mut aliases = Vec::<proc_macro2::TokenStream>::new();
    for variant in &enumeration.variants {
        let name = &variant.ident;
        let literal = Literal::string(&name.to_string().to_lowercase());
        let mut stream = quote! { #literal };
        for attr in &variant.attrs {
            let Meta::List(ref list) = attr.meta else {
                continue;
            };
            let mut iter = list.tokens.clone().into_iter();
            while let Some(TokenTree::Ident(ref ident)) = iter.next() {
                let SupportedArguments::Alias = ident
                    .to_string()
                    .parse::<SupportedArguments>()
                    .map_err(|_| {
                        new_error(ident, &format!("unknown variant attribute `{ident}"))
                    })?
                else {
                    continue;
                };
                if let Some(TokenTree::Punct(ref punct)) = iter.next() {
                    if let Some(TokenTree::Literal(ref alias)) = iter.next() {
                        if alias.to_string() == "\"\"" {
                            return Err(new_error(&alias, "expected non empty literal"));
                        }
                        let Some(TokenTree::Punct(_)) = iter.next() else {
                            return Err(new_error(
                                &alias,
                                &format!("expected punct `,` after `{alias}`"),
                            ));
                        };
                        stream.extend(quote! { | #alias });
                    } else if punct.to_string() != "=" {
                        return Err(new_error(
                            &punct,
                            &format!("expected punct `=` after `{ident}`"),
                        ));
                    } else {
                        return Err(new_error(
                            &punct,
                            &format!("expected literal after `{punct}`"),
                        ));
                    }
                } else {
                    return Err(new_error(
                        &ident,
                        &format!("expected punct `=` after `{ident}`"),
                    ));
                }
            }
        }
        stream.extend(quote! { => Ok(Self::#name) });
        aliases.push(stream);
    }
    Ok(aliases)
}

// TODO: better error handling
fn parse_roles(enumeration: &DataEnum) -> syn::Result<HashMap<syn::Ident, Vec<Literal>>> {
    let mut map = HashMap::<syn::Ident, Vec<Literal>>::new();
    for variant in &enumeration.variants {
        let mut roles = Vec::<Literal>::new();
        for attr in &variant.attrs {
            let Meta::List(ref list) = attr.meta else {
                continue;
            };
            let mut iter = list.tokens.clone().into_iter();
            while let Some(token) = iter.next() {
                if let TokenTree::Ident(ref ident) = token {
                    let SupportedArguments::Roles = ident
                        .to_string()
                        .parse::<SupportedArguments>()
                        .map_err(|_| {
                            new_error(ident, &format!("unknown variant attribute `{ident}"))
                        })?
                    else {
                        continue;
                    };
                    if let Some(token) = iter.next() {
                        let TokenTree::Punct(ref equal) = token else {
                            return Err(new_error(
                                &ident,
                                &format!("expected punct `=` after {ident}"),
                            ));
                        };
                        if equal.to_string() != "=" {
                            return Err(new_error(
                                &equal,
                                &format!("expected punct `=` after {ident}"),
                            ));
                        }
                        if let Some(token) = iter.next() {
                            if let TokenTree::Group(ref group) = token {
                                if let proc_macro2::Delimiter::Bracket = group.delimiter() {
                                    for token in group.stream() {
                                        if let TokenTree::Literal(literal) = token {
                                            // TODO: is pushing this before the if let okay?
                                            roles.push(literal);
                                            if let Some(ref token) = iter.next() {
                                                if let TokenTree::Punct(_) = token {
                                                    continue;
                                                } else {
                                                    // return Err(new_error(
                                                    //     &group,
                                                    //     &format!("unexpected token"),
                                                    // ));
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    // return Err(new_error(
                                    //     equal,
                                    //     &format!("expected `[` after `{equal}`"),
                                    // ));
                                }
                            } else {
                                return Err(new_error(
                                    equal,
                                    &format!("expected `[` after `{equal}`"),
                                ));
                            }
                        }
                    }
                }
            }
        }
        map.insert(variant.ident.clone(), roles);
    }
    Ok(map)
}

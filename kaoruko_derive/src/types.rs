#![allow(unused)]

use proc_macro::TokenStream;
use proc_macro2::Literal;
use syn::Ident;

pub type Variant = Ident;

#[derive(Default)]
pub(crate) struct Config {
    pub(crate) alias: Option<Literal>,
    pub(crate) description: Option<Literal>,
    pub(crate) roles: Option<Vec<Literal>>,
    pub(crate) string_options: Option<StringOption>,
}

#[derive(Default)]
pub(crate) struct StringOption {
    pub(crate) required: bool,
    pub(crate) allow_whitespaces: bool,
}

#[derive(Debug, Clone)]
pub(crate) enum SupportedArguments {
    Alias,
    Description,
    Roles,
    StringOptions,
    Unknown,
}

impl std::str::FromStr for SupportedArguments {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "alias" => Ok(Self::Alias),
            "description" => Ok(Self::Description),
            "roles" => Ok(Self::Roles),
            "string_options" => Ok(Self::StringOptions),
            _ => Err(()),
        }
    }
}

pub(crate) enum SupportedStringArguments {
    Required,
    AllowWhitespaces,
}

impl std::str::FromStr for SupportedStringArguments {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "required" => Ok(Self::Required),
            "allow_whitespaces" => Ok(Self::AllowWhitespaces),
            _ => Err(()),
        }
    }
}

pub(crate) enum Roles {
    Anyone,
    Developer,
    Creator,
}

impl std::str::FromStr for Roles {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "anyone" => Ok(Self::Anyone),
            "developer" => Ok(Self::Developer),
            "creator" => Ok(Self::Creator),
            _ => Err(()),
        }
    }
}

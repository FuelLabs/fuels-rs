use proc_macro2::Ident;
use syn::{
    parse::ParseStream, parse_macro_input::ParseMacroInput, punctuated::Punctuated, AttributeArgs,
    Error, Meta::List, MetaList, NestedMeta, NestedMeta::Meta,
};

#[derive(Debug)]
pub struct Command {
    pub name: Ident,
    pub contents: Punctuated<NestedMeta, syn::token::Comma>,
}

impl Command {
    pub fn parse_multiple(input: ParseStream) -> syn::Result<Vec<Command>> {
        AttributeArgs::parse(input)?
            .into_iter()
            .map(Command::new)
            .collect()
    }

    pub fn new(nested_meta: NestedMeta) -> syn::Result<Self> {
        if let Meta(List(MetaList { path, nested, .. })) = nested_meta {
            let name = path.get_ident().cloned().ok_or_else(|| {
                Error::new_spanned(path, "Command name cannot be a Path -- i.e. contain ':'.")
            })?;
            Ok(Self {
                name,
                contents: nested,
            })
        } else {
            Err(Error::new_spanned(
                nested_meta,
                "Expected a command name literal -- e.g. `Something(...)`",
            ))
        }
    }

    #[cfg(test)]
    pub(crate) fn parse_multiple_from_token_stream(
        stream: proc_macro2::TokenStream,
    ) -> syn::Result<Vec<Self>> {
        syn::parse::Parser::parse2(Command::parse_multiple, stream)
    }

    #[cfg(test)]
    pub(crate) fn parse_single_from_token_stream(
        stream: proc_macro2::TokenStream,
    ) -> syn::Result<Self> {
        syn::parse::Parser::parse2(Command::parse_multiple, stream.clone())?
            .pop()
            .ok_or_else(|| Error::new_spanned(stream, "Expected to have at least one command!"))
    }
}
#[cfg(test)]
mod tests {
    use quote::quote;

    use crate::parse_utils::command::Command;

    #[test]
    fn command_name_is_properly_extracted() -> syn::Result<()> {
        // given
        let macro_contents = quote! {SomeCommand(), OtherCommand()};

        // when
        let commands = Command::parse_multiple_from_token_stream(macro_contents)?;

        // then
        let command_names = commands
            .into_iter()
            .map(|command| command.name.to_string())
            .collect::<Vec<_>>();

        assert_eq!(command_names, vec!["SomeCommand", "OtherCommand"]);

        Ok(())
    }
}

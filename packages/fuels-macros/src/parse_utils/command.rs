use proc_macro2::{Ident, TokenStream};
use syn::{
    Error, Meta,
    Meta::List,
    parse::{ParseStream, Parser},
    punctuated::Punctuated,
    token::Comma,
};

#[derive(Debug)]
pub struct Command {
    pub name: Ident,
    pub contents: TokenStream,
}

impl Command {
    pub fn parse_multiple(input: ParseStream) -> syn::Result<Vec<Command>> {
        input
            .call(Punctuated::<Meta, Comma>::parse_terminated)?
            .into_iter()
            .map(Command::new)
            .collect()
    }

    pub fn new(meta: Meta) -> syn::Result<Self> {
        if let List(meta_list) = meta {
            let name = meta_list.path.get_ident().cloned().ok_or_else(|| {
                Error::new_spanned(
                    &meta_list.path,
                    "command name cannot be a Path -- i.e. contain ':'",
                )
            })?;

            Ok(Self {
                name,
                contents: meta_list.tokens,
            })
        } else {
            Err(Error::new_spanned(
                meta,
                "expected a command name literal -- e.g. `Something(...)`",
            ))
        }
    }

    pub fn parse_nested_metas(self) -> syn::Result<Punctuated<Meta, Comma>> {
        Punctuated::<Meta, Comma>::parse_terminated.parse2(self.contents)
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
            .ok_or_else(|| Error::new_spanned(stream, "expected to have at least one command!"))
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

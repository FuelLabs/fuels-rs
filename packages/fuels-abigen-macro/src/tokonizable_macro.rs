use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DataEnum, DataStruct, DeriveInput, Error};

pub fn generate_tokenizable_impl(input: DeriveInput) -> syn::Result<TokenStream> {
    match &input.data {
        Data::Struct(struct_contents) => tokenizable_struct(&input, struct_contents),
        Data::Enum(enum_contents) => tokenizable_enum(&input, enum_contents),
        _ => {
            panic!("Union type is not supported")
        }
    }
}

fn tokenizable_struct(
    input: &DeriveInput,
    struct_contents: &DataStruct,
) -> Result<TokenStream, Error> {
    let struct_name = &input.ident;

    let (impl_gen, type_gen, where_clause) = input.generics.split_for_impl();

    let struct_name_str = struct_name.to_string();
    let from_token_calls = &struct_contents
        .fields
        .iter()
        .filter_map(|field| field.ident.as_ref())
        .map(|ident| {
            quote! {#ident: ::fuels::types::traits::Tokenizable::from_token(next_token()?)?}
        })
        .collect::<Vec<_>>();

    let into_token_calls = &struct_contents
        .fields
        .iter()
        .filter_map(|field| field.ident.as_ref())
        .map(|ident| {
            quote! {self.#ident.into_token()}
        })
        .collect::<Vec<_>>();

    Ok(quote! {
        impl #impl_gen ::fuels::types::traits::Tokenizable for #struct_name #type_gen #where_clause {
            fn into_token(self) -> ::fuels::types::Token {
                let tokens = [#(#into_token_calls),*].to_vec();
                ::fuels::types::Token::Struct(tokens)
            }

            fn from_token(token: ::fuels::types::Token)  -> ::std::result::Result<Self, ::fuels::types::errors::Error> {
                match token {
                    ::fuels::types::Token::Struct(tokens) => {
                        let mut tokens_iter = tokens.into_iter();
                        let mut next_token = move || { tokens_iter
                            .next()
                            .ok_or_else(|| { ::fuels::types::errors::Error::InstantiationError(format!("Ran out of tokens before '{}' has finished construction!", #struct_name_str)) })
                        };
                        ::std::result::Result::Ok(Self { #( #from_token_calls, )* })
                    },
                    other => ::std::result::Result::Err(::fuels::types::errors::Error::InstantiationError(format!("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}", #struct_name_str, other))),
                }
            }
        }
    })
}

fn tokenizable_enum(_input: &DeriveInput, _enum_contents: &DataEnum) -> Result<TokenStream, Error> {
    todo!()
}

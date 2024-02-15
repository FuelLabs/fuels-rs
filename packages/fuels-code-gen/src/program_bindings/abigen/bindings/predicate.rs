use fuel_abi_types::abi::full_program::FullProgramABI;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::{
    error::Result,
    program_bindings::{
        abigen::{
            bindings::{function_generator::FunctionGenerator, utils::extract_main_fn},
            configurables::generate_code_for_configurable_constants,
        },
        generated_code::GeneratedCode,
    },
    utils::{ident, TypePath},
};

pub(crate) fn predicate_bindings(
    name: &Ident,
    abi: FullProgramABI,
    no_std: bool,
) -> Result<GeneratedCode> {
    let encode_function = expand_fn(&abi)?;
    let encoder_struct_name = ident(&format!("{name}Encoder"));

    let configuration_struct_name = ident(&format!("{name}Configurables"));
    let constant_configuration_code =
        generate_code_for_configurable_constants(&configuration_struct_name, &abi.configurables)?;

    let code = quote! {
        #[derive(Default)]
        pub struct #encoder_struct_name{
            encoder: ::fuels::core::codec::ABIEncoder,
        }

        impl #encoder_struct_name {
           #encode_function

            pub fn new(encoder_config: ::fuels::core::codec::EncoderConfig) -> Self {
                Self {
                    encoder: ::fuels::core::codec::ABIEncoder::new(encoder_config)
                }
            }
        }

        #constant_configuration_code
    };
    // All publicly available types generated above should be listed here.
    let type_paths = [&encoder_struct_name, &configuration_struct_name]
        .map(|type_name| TypePath::new(type_name).expect("We know the given types are not empty"))
        .into_iter()
        .collect();

    Ok(GeneratedCode::new(code, type_paths, no_std))
}

fn expand_fn(abi: &FullProgramABI) -> Result<TokenStream> {
    let fun = extract_main_fn(&abi.functions)?;
    let mut generator = FunctionGenerator::new(fun)?;

    let arg_tokens = generator.tokenized_args();

    let body = quote! {
        self.encoder.encode(&#arg_tokens)
    };
    let output_type = quote! {
        ::fuels::types::errors::Result<::fuels::types::unresolved_bytes::UnresolvedBytes>
    };

    generator
        .set_doc("Run the predicate's encode function with the provided arguments".to_string())
        .set_name("encode_data".to_string())
        .set_output_type(output_type)
        .set_body(body);

    Ok(generator.generate())
}

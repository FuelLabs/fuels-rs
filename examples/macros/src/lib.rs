extern crate alloc;

#[cfg(test)]
mod tests {
    use fuels::prelude::*;

    #[test]
    fn example_of_abigen_usage() {
        // ANCHOR: multiple_abigen_program_types
        abigen!(
            Contract(name="ContractA", abi="packages/fuels/tests/bindings/sharing_types/contract_a/out/release/contract_a-abi.json"),
            Contract(name="ContractB", abi="packages/fuels/tests/bindings/sharing_types/contract_b/out/release/contract_b-abi.json"),
            Script(name="MyScript", abi="packages/fuels/tests/scripts/arguments/out/release/arguments-abi.json"),
            Predicate(name="MyPredicateEncoder", abi="packages/fuels/tests/predicates/basic_predicate/out/release/basic_predicate-abi.json"),
        );
        // ANCHOR_END: multiple_abigen_program_types
    }

    #[test]
    fn macro_deriving() {
        // ANCHOR: deriving_traits
        use fuels::macros::{Parameterize, Tokenizable};

        #[derive(Parameterize, Tokenizable)]
        struct MyStruct {
            field_a: u8,
        }

        #[derive(Parameterize, Tokenizable)]
        enum SomeEnum {
            A(MyStruct),
            B(Vec<u64>),
        }
        // ANCHOR_END: deriving_traits
    }
    #[test]
    fn macro_deriving_extra() {
        {
            use fuels::{
                core as fuels_core_elsewhere,
                macros::{Parameterize, Tokenizable},
                types as fuels_types_elsewhere,
            };

            // ANCHOR: deriving_traits_paths
            #[derive(Parameterize, Tokenizable)]
            #[FuelsCorePath = "fuels_core_elsewhere"]
            #[FuelsTypesPath = "fuels_types_elsewhere"]
            pub struct SomeStruct {
                field_a: u64,
            }
            // ANCHOR_END: deriving_traits_paths
        }
        {
            // ANCHOR: deriving_traits_nostd
            use fuels::macros::{Parameterize, Tokenizable};
            #[derive(Parameterize, Tokenizable)]
            #[NoStd]
            pub struct SomeStruct {
                field_a: u64,
            }
            // ANCHOR_END: deriving_traits_nostd
        }
    }
}

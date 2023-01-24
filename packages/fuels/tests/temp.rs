#![allow(warnings)]
use fuels::{
    fuels_abigen::Parameterize,
    types::traits::{Parameterize, Tokenizable},
};
use fuels_abigen_macro::Tokenizable;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameterize_for_enum() {
        #[derive(Parameterize, Tokenizable)]
        struct H<G: Parameterize + Tokenizable> {
            s: G,
        }

        #[derive(Parameterize, Tokenizable)]
        enum SomeEnum<T: Parameterize + Tokenizable> {
            h(H<bool>),
            a(T),
            e,
        }

        #[derive(Parameterize, Tokenizable)]
        enum SomeEnum2 {
            c,
        }

        let token = SomeEnum::<u64>::e.into_token();
        dbg!(SomeEnum::<u64>::param_type());
    }
}

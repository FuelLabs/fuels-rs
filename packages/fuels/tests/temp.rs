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
            b(),
            d(()),
        }

        #[derive(Parameterize, Tokenizable)]
        enum SomeEnum2 {
            c,
        }

        let ed = SomeEnum::<u64>::d(());
        let eb = SomeEnum::<u64>::b();

        dbg!(SomeEnum::<u64>::param_type());
        dbg!(ed.into_token());
        dbg!(eb.into_token());
    }
}

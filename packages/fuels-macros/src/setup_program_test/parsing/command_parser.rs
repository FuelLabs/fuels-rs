macro_rules! command_parser {
    ($($command_name: ident -> $command_struct: ty),+ $(,)?) => {
        #[derive(Default)]
        #[allow(non_snake_case)]
        pub(crate) struct CommandParser {
            $(pub(crate) $command_name: Vec<$command_struct>),*
        }

        impl CommandParser {
            fn available_commands() -> impl Iterator<Item=&'static str> {
                [$(stringify!($command_name)),*].into_iter()
            }

            pub(crate) fn parse_and_save(&mut self, command: $crate::parse_utils::Command) -> ::syn::Result<()>{
                match command.name.to_string().as_str() {
                    $(stringify!($command_name) => self.$command_name.push(command.try_into()?),)*
                    _ => {
                        let msg = Self::available_commands().map(|command| format!("'{command}'")).join(", ");
                        return Err(::syn::Error::new(command.name.span(), format!("Unrecognized command. Expected one of: {msg}")));
                    }
                };
                Ok(())
            }
        }

        impl Parse for CommandParser {
            fn parse(input: ::syn::parse::ParseStream) -> Result<Self> {
                use $crate::parse_utils::ErrorsExt;
                let mut command_parser = Self::default();

                let mut errors = vec![];
                for command in $crate::parse_utils::Command::parse_multiple(input)? {
                    if let Err(error) = command_parser.parse_and_save(command) {
                        errors.push(error);
                    }
                }

                errors.into_iter().validate_no_errors()?;
                Ok(command_parser)
            }
        }
    }
}
pub(crate) use command_parser;

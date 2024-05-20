use super::deps::All;
use itertools::Itertools;
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Command {
    Custom {
        program: String,
        args: Vec<String>,
        env: Vec<(String, String)>,
        deps: All,
    },
    MdCheck,
}

impl Command {
    pub fn deps(&self) -> All {
        match self {
            Self::Custom { deps, .. } => deps.clone(),
            Self::MdCheck => All::default(),
        }
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Custom {
                program, args, env, ..
            } => {
                let args = args.iter().join(" ");
                if env.is_empty() {
                    write!(f, "{program} {args}")
                } else {
                    let env = env
                        .iter()
                        .map(|(key, value)| format!("{key}='{value}'"))
                        .join(" ");
                    write!(f, "{env} {program} {args}")
                }
            }
            Self::MdCheck { .. } => write!(f, "MdCheck"),
        }
    }
}

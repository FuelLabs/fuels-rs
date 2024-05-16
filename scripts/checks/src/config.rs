pub mod description;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct TasksDescription {
    pub run_for_dirs: Vec<PathBuf>,
    pub commands: Vec<Command>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub enum RunIf {
    CwdDoesntEndWith(Vec<String>),
}

impl RunIf {
    pub fn should_run(&self, cwd: &Path) -> bool {
        match self {
            RunIf::CwdDoesntEndWith(suffixes) => {
                !suffixes.iter().any(|suffix| cwd.ends_with(suffix))
            }
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub enum Command {
    MdCheck {
        run_if: Option<RunIf>,
    },
    Custom {
        cmd: Vec<String>,
        env: Option<HashMap<String, String>>,
        run_if: Option<RunIf>,
    },
}

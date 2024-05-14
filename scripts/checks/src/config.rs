use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, serde::Deserialize)]
pub struct Config(pub Vec<Group>);

#[derive(Debug, serde::Deserialize)]
pub struct Group {
    pub run_for_dirs: Vec<PathBuf>,
    pub commands: Vec<Command>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub enum Command {
    MdCheck {
        ignore_if_in_dir: Option<Vec<String>>,
    },
    Custom {
        ignore_if_cwd_ends_with: Option<Vec<String>>,
        cmd: Vec<String>,
        env: Option<HashMap<String, String>>,
    },
}

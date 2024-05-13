use std::path::PathBuf;

#[derive(Debug, serde::Deserialize)]
pub struct Config(pub Vec<Group>);

#[derive(Debug, serde::Deserialize)]
pub struct Group {
    pub working_dir: PathBuf,
    pub name: String,
    pub commands: Vec<Command>,
}

#[derive(Debug, serde::Deserialize)]
pub enum Command {
    MdCheck { ignore: Vec<PathBuf> },
    Custom(Vec<String>),
}

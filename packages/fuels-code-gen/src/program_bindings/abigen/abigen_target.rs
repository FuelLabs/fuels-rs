use std::{
    convert::TryFrom,
    env, fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use fuel_abi_types::abi::full_program::FullProgramABI;
use proc_macro2::Ident;

use crate::error::{error, Error, Result};

#[derive(Debug, Clone)]
pub struct AbigenTarget {
    pub(crate) name: String,
    pub(crate) source: Abi,
    pub(crate) program_type: ProgramType,
}

impl AbigenTarget {
    pub fn new(name: String, source: Abi, program_type: ProgramType) -> Self {
        Self {
            name,
            source,
            program_type,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn source(&self) -> &Abi {
        &self.source
    }

    pub fn program_type(&self) -> ProgramType {
        self.program_type
    }
}

#[derive(Debug, Clone)]
pub struct Abi {
    pub(crate) path: Option<PathBuf>,
    pub(crate) abi: FullProgramABI,
}

impl Abi {
    pub fn load_from(path: impl AsRef<Path>) -> Result<Abi> {
        let path = Self::canonicalize_path(path.as_ref())?;

        let json_abi = fs::read_to_string(&path).map_err(|e| {
            error!(
                "failed to read `abi` file with path {}: {}",
                path.display(),
                e
            )
        })?;
        let abi = Self::parse_from_json(&json_abi)?;

        Ok(Abi {
            path: Some(path),
            abi,
        })
    }

    fn canonicalize_path(path: &Path) -> Result<PathBuf> {
        let current_dir = env::current_dir()
            .map_err(|e| error!("unable to get current directory: ").combine(e))?;

        let root = current_dir.canonicalize().map_err(|e| {
            error!(
                "unable to canonicalize current directory {}: ",
                current_dir.display()
            )
            .combine(e)
        })?;

        let path = root.join(path);

        if path.is_relative() {
            path.canonicalize().map_err(|e| {
                error!(
                    "unable to canonicalize file from working dir {} with path {}: {}",
                    env::current_dir()
                        .map(|cwd| cwd.display().to_string())
                        .unwrap_or_else(|err| format!("??? ({err})")),
                    path.display(),
                    e
                )
            })
        } else {
            Ok(path)
        }
    }

    fn parse_from_json(json_abi: &str) -> Result<FullProgramABI> {
        FullProgramABI::from_json_abi(json_abi)
            .map_err(|e| error!("malformed `abi`. Did you use `forc` to create it?: ").combine(e))
    }

    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    pub fn abi(&self) -> &FullProgramABI {
        &self.abi
    }
}

impl FromStr for Abi {
    type Err = Error;

    fn from_str(json_abi: &str) -> Result<Self> {
        let abi = Abi::parse_from_json(json_abi)?;

        Ok(Abi { path: None, abi })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramType {
    Script,
    Contract,
    Predicate,
}

impl FromStr for ProgramType {
    type Err = Error;

    fn from_str(string: &str) -> std::result::Result<Self, Self::Err> {
        let program_type = match string {
            "Script" => ProgramType::Script,
            "Contract" => ProgramType::Contract,
            "Predicate" => ProgramType::Predicate,
            _ => {
                return Err(error!(
                    "`{string}` is not a valid program type. Expected one of: `Script`, `Contract`, `Predicate`"
                ))
            }
        };

        Ok(program_type)
    }
}

impl TryFrom<Ident> for ProgramType {
    type Error = syn::Error;

    fn try_from(ident: Ident) -> std::result::Result<Self, Self::Error> {
        ident
            .to_string()
            .as_str()
            .parse()
            .map_err(|e| Self::Error::new(ident.span(), e))
    }
}

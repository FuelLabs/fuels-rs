use std::{
    borrow::Cow,
    env, fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::error::{error, Error, Result};

/// A source of a Truffle artifact JSON.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Source {
    /// A raw ABI string
    String(String),

    /// An ABI located on the local file system.
    Local(PathBuf),
}

impl Source {
    /// Parses an ABI from a source
    ///
    /// Contract ABIs can be retrieved from the local filesystem or it can
    /// be provided in-line. It accepts:
    ///
    /// - raw ABI JSON
    ///
    /// - `relative/path/to/Contract.json`: a relative path to an ABI JSON file.
    /// This relative path is rooted in the current working directory.
    /// To specify the root for relative paths, use `Source::with_root`.
    ///
    /// - `/absolute/path/to/Contract.json to an ABI JSON file.
    pub fn parse<S>(source: S) -> Result<Self>
    where
        S: AsRef<str>,
    {
        let source = source.as_ref().trim();

        if source.starts_with('{') || source.starts_with('[') || source.starts_with('\n') {
            return Ok(Source::String(source.to_owned()));
        }
        let root = env::current_dir()?.canonicalize()?;
        Ok(Source::with_root(root, source))
    }

    /// Parses an artifact source from a string and a specified root directory
    /// for resolving relative paths. See `Source::with_root` for more details
    /// on supported source strings.
    fn with_root<P, S>(root: P, source: S) -> Self
    where
        P: AsRef<Path>,
        S: AsRef<str>,
    {
        Source::local(root.as_ref().join(source.as_ref()))
    }

    /// Creates a local filesystem source from a path string.
    fn local<P>(path: P) -> Self
    where
        P: AsRef<Path>,
    {
        Source::Local(path.as_ref().into())
    }

    /// Retrieves the source JSON of the artifact this will either read the JSON
    /// from the file system or retrieve a contract ABI from the network
    /// depending on the source type.
    pub fn get(&self) -> Result<String> {
        match self {
            Source::Local(path) => get_local_contract(path),
            Source::String(abi) => Ok(abi.clone()),
        }
    }

    pub fn path(&self) -> Option<PathBuf> {
        match self {
            Source::Local(path) => Some(path.clone()),
            _ => None,
        }
    }
}

fn get_local_contract(path: &Path) -> Result<String> {
    let path = if path.is_relative() {
        let absolute_path = path.canonicalize().map_err(|e| {
            error!(
                "unable to canonicalize file from working dir {} with path {}",
                env::current_dir()
                    .map(|cwd| cwd.display().to_string())
                    .unwrap_or_else(|err| format!("??? ({err})")),
                path.display()
            )
            .combine(e)
        })?;
        Cow::Owned(absolute_path)
    } else {
        Cow::Borrowed(path)
    };

    let json = fs::read_to_string(&path).map_err(|e| {
        error!(
            "failed to read artifact JSON file with path {}",
            path.display()
        )
        .combine(e)
    })?;
    Ok(json)
}

impl FromStr for Source {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Source::parse(s)
    }
}

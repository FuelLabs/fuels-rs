use crate::{
    cli,
    tasks::{builder::Builder, deps::CiJob, Tasks},
};
use std::path::PathBuf;

pub fn ci_jobs(workspace_root: PathBuf) -> Vec<CiJob> {
    let tasks = normal(workspace_root);
    tasks.ci_jobs()
}

pub fn choose_tasks(cli: &cli::Cli) -> Tasks {
    let mut tasks = match cli.flavor {
        cli::Flavor::Normal => normal(cli.root.clone()),
        cli::Flavor::HackFeatures => hack_features(cli.root.clone()),
        cli::Flavor::HackDeps => hack_deps(cli.root.clone()),
    };

    if let Some(ids) = &cli.only_tasks_with_ids {
        tasks.retain_with_ids(ids);
    }

    if let Some(dirs) = &cli.only_tasks_in_dir {
        tasks.retain_with_dirs(dirs);
    }

    if !cli.sway_type_paths {
        tasks.retain_without_type_paths();
    }

    tasks
}

fn normal(workspace_root: PathBuf) -> Tasks {
    let mut builder = Builder::new(workspace_root, &["-Dwarnings"]);

    builder.common();
    builder.e2e_specific();
    builder.wasm_specific();
    builder.workspace_level();

    builder.build()
}

fn hack_features(workspace_root: PathBuf) -> Tasks {
    let mut builder = Builder::new(workspace_root, &["-Dwarnings"]);

    builder.hack_features_common();
    builder.hack_features_e2e();

    builder.build()
}

fn hack_deps(workspace_root: PathBuf) -> Tasks {
    let mut builder = Builder::new(workspace_root, &["-Dwarnings"]);

    builder.hack_deps_common();

    builder.build()
}

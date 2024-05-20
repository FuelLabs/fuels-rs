use nix::unistd::Pid;

pub fn watch_for_cancel(cancel_token: tokio_util::sync::CancellationToken) {
    tokio::task::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        cancel_token.cancel();
    });
}

pub fn configure_child_process_cleanup() -> anyhow::Result<()> {
    // This process is moved into its own process group so that it's easier to kill any of its children.
    nix::unistd::setpgid(Pid::from_raw(0), Pid::from_raw(0))?;
    Ok(())
}

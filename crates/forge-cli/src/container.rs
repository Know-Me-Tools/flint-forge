//! Re-exec the CLI inside the `flint-forge-cli` container when `--container` is set.

use anyhow::{bail, Context, Result};

use crate::cli::{Cli, Commands};
use crate::commands::register::RegisterArgs;

pub(crate) const INSIDE_CONTAINER_ENV: &str = "FORGE_CONTAINERIZED";
const CONTAINER_IMAGE: &str = "flint-forge-cli";

pub(crate) async fn run_in_container(cli: &Cli) -> Result<()> {
    let mut cmd = tokio::process::Command::new("docker");
    cmd.arg("run").arg("--rm").arg("-i");

    cmd.env(INSIDE_CONTAINER_ENV, "1");

    for key in [
        "DATABASE_URL",
        "KILN_ADMIN_URL",
        "FLINT_JWT_SECRET",
        "RUST_LOG",
        "RUST_BACKTRACE",
    ] {
        if let Ok(value) = std::env::var(key) {
            cmd.env(key, value);
        }
    }

    let cwd = std::env::current_dir().context("current directory not available")?;
    let cwd_str = cwd.to_string_lossy();
    cmd.arg("-v")
        .arg(format!("{cwd_str}:{cwd_str}"))
        .arg("-w")
        .arg(cwd_str.as_ref());

    if let Commands::Function {
        command: crate::cli::FunctionCommands::Register(RegisterArgs { path, .. }),
    } = &cli.command
    {
        if let Some(parent) = path.parent() {
            let parent_str = parent.to_string_lossy();
            cmd.arg("-v").arg(format!("{parent_str}:{parent_str}"));
        }
    }

    cmd.arg(CONTAINER_IMAGE);
    cmd.arg("forge");

    for token in std::env::args().skip(1) {
        if token == "--container" || token.starts_with("--container=") {
            continue;
        }
        cmd.arg(token);
    }

    let status = cmd
        .status()
        .await
        .with_context(|| "failed to run docker command")?;
    if !status.success() {
        bail!("container command failed with status {status}");
    }
    Ok(())
}

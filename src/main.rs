use std::fs::File;

use anyhow::{Context, Result, bail};
use clap::Parser;

mod build_info;
mod cli;
mod commands;
mod console;
mod workspace;

fn main() -> Result<()> {
    let args = cli::Args::parse();
    let file = File::open(&args.file)
        .with_context(|| format!("Cannot open file: {}", args.file.display()))?;
    let metadata = file.metadata().context("Cannot read file metadata")?;
    if !metadata.is_file() {
        bail!("Expected a regular file: {}", args.file.display());
    }
    // Only metadata is read; dump contents are not loaded.
    drop(file);
    let workspace =
        workspace::Workspace::prepare(args.workspace.as_deref(), &args.file, args.force)?;
    console::run(&args.file, metadata.len(), &workspace)
}

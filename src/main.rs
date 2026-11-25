use std::{fs::File, path::PathBuf};

use anyhow::{Context, Result, bail};
use clap::Parser;

mod console;

/// Java Heap Dump Analyzer
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Path to a Java heap dump file
    file: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let file = File::open(&args.file)
        .with_context(|| format!("Cannot open file: {}", args.file.display()))?;
    let metadata = file.metadata().context("Cannot read file metadata")?;
    if !metadata.is_file() {
        bail!("Expected a regular file: {}", args.file.display());
    }
    // Only metadata is read; dump contents are not loaded.
    drop(file);
    console::run(&args.file, metadata.len())
}

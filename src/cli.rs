use std::path::PathBuf;

use clap::Parser;

/// Java Heap Dump Analyzer
#[derive(Debug, Parser)]
#[command(version, about)]
pub(crate) struct Args {
    /// Path to a Java heap dump file
    pub(crate) file: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    #[test]
    fn help_flags_work_without_a_file() {
        for flag in ["-h", "--help"] {
            let error = Args::try_parse_from(["jda", flag]).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::DisplayHelp);
            let help = error.to_string();
            assert!(help.contains("-h, --help"));
            assert!(help.contains("-V, --version"));
        }
    }

    #[test]
    fn version_flags_work_without_a_file() {
        for flag in ["-V", "--version"] {
            let error = Args::try_parse_from(["jda", flag]).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::DisplayVersion);
            assert_eq!(
                error.to_string().trim(),
                format!("jda {}", env!("CARGO_PKG_VERSION"))
            );
        }
    }

    #[test]
    fn normal_startup_requires_and_preserves_the_file_path() {
        let error = Args::try_parse_from(["jda"]).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::MissingRequiredArgument);
        let args = Args::try_parse_from(["jda", "dump files/app.hprof"]).unwrap();
        assert_eq!(args.file, PathBuf::from("dump files/app.hprof"));
    }
}

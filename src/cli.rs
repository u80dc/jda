use std::path::PathBuf;

use clap::Parser;

/// Java Heap Dump Analyzer
#[derive(Debug, Parser)]
#[command(version = crate::build_info::VERSION, about)]
pub(crate) struct Args {
    /// Path to a Java heap dump file
    pub(crate) file: PathBuf,

    /// Root directory for dump workspaces (defaults to user temporary directory/.jda)
    #[arg(short = 'w', long, value_name = "PATH")]
    pub(crate) workspace: Option<PathBuf>,

    /// Clear existing analysis files in this dump workspace before starting
    #[arg(short = 'f', long)]
    pub(crate) force: bool,
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
            assert!(help.contains("-w, --workspace <PATH>"));
            assert!(help.contains("-f, --force"));
        }
    }

    #[test]
    fn version_flags_work_without_a_file() {
        for flag in ["-V", "--version"] {
            let error = Args::try_parse_from(["jda", flag]).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::DisplayVersion);
            assert_eq!(
                error.to_string().trim(),
                format!(
                    "jda {} ({})",
                    env!("CARGO_PKG_VERSION"),
                    env!("JDA_BUILD_TARGET")
                )
            );
        }
    }

    #[test]
    fn normal_startup_requires_and_preserves_the_file_path() {
        let error = Args::try_parse_from(["jda"]).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::MissingRequiredArgument);
        let args = Args::try_parse_from(["jda", "dump files/app.hprof"]).unwrap();
        assert_eq!(args.file, PathBuf::from("dump files/app.hprof"));
        assert!(args.workspace.is_none());
        assert!(!args.force);
    }

    #[test]
    fn workspace_flags_preserve_paths_before_or_after_the_file() {
        for flag in ["-w", "--workspace"] {
            for path in ["work", ".", r"D:\jda work\分析"] {
                for arguments in [
                    ["jda", flag, path, "app.hprof"],
                    ["jda", "app.hprof", flag, path],
                ] {
                    let args = Args::try_parse_from(arguments).unwrap();
                    assert_eq!(args.file, PathBuf::from("app.hprof"));
                    assert_eq!(args.workspace, Some(PathBuf::from(path)));
                }
            }
        }
    }

    #[test]
    fn workspace_flags_require_a_path() {
        for flag in ["-w", "--workspace"] {
            assert!(Args::try_parse_from(["jda", "app.hprof", flag]).is_err());
        }
    }

    #[test]
    fn force_flags_work_with_workspace_and_file_arguments() {
        for flag in ["-f", "--force"] {
            for arguments in [
                ["jda", flag, "app.hprof", "-w", "work"],
                ["jda", "--workspace", "work", "app.hprof", flag],
            ] {
                let args = Args::try_parse_from(arguments).unwrap();
                assert!(args.force);
                assert_eq!(args.file, PathBuf::from("app.hprof"));
                assert_eq!(args.workspace, Some(PathBuf::from("work")));
            }
        }
        let error = Args::try_parse_from(["jda", "--force"]).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn help_and_version_flags_exit_even_when_force_is_requested() {
        for (flag, expected_kind) in [
            ("-h", ErrorKind::DisplayHelp),
            ("--help", ErrorKind::DisplayHelp),
            ("-V", ErrorKind::DisplayVersion),
            ("--version", ErrorKind::DisplayVersion),
        ] {
            let error = Args::try_parse_from(["jda", "--force", "-w", "work", flag]).unwrap_err();
            assert_eq!(error.kind(), expected_kind);
        }
    }
}

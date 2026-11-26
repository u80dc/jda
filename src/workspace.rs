use std::{
    env, fs, io,
    path::{self, Path, PathBuf},
};

use anyhow::{Context, Result, bail};

/// Directory for persistent analysis artifacts associated with one dump filename.
#[derive(Debug)]
pub(crate) struct Workspace {
    path: PathBuf,
}

impl Workspace {
    pub(crate) fn prepare(base: Option<&Path>, dump: &Path, force: bool) -> Result<Self> {
        let path = resolve_path(base, dump)?;
        let expected_root = path.parent().context("Workspace has no parent")?;
        if force {
            // Check existing links before create_dir_all could follow them.
            reject_redirected_directory(expected_root)?;
            reject_redirected_directory(&path)?;
        }
        fs::create_dir_all(&path)
            .with_context(|| format!("Cannot create analysis workspace: {}", path.display()))?;
        if force {
            clear_contents(&path, expected_root, dump)?;
        }
        Ok(Self { path })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

fn resolve_path(base: Option<&Path>, dump: &Path) -> Result<PathBuf> {
    let filename = dump
        .file_name()
        .with_context(|| format!("Dump file path has no valid filename: {}", dump.display()))?;
    Ok(resolve_root(base)?.join(filename))
}

fn resolve_root(base: Option<&Path>) -> Result<PathBuf> {
    let root = match base {
        Some(base) if base.as_os_str().is_empty() => bail!("Workspace base path cannot be empty"),
        Some(base) => base.to_path_buf(),
        None => env::temp_dir().join(".jda"),
    };
    path::absolute(&root)
        .with_context(|| format!("Cannot resolve workspace root: {}", root.display()))
}
fn reject_redirected_directory(directory: &Path) -> Result<()> {
    match fs::symlink_metadata(directory) {
        Ok(metadata) if is_redirected(&metadata) => {
            bail!(
                "Refusing to clear a linked workspace directory: {}",
                directory.display()
            );
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| {
            format!(
                "Cannot inspect workspace directory: {}",
                directory.display()
            )
        }),
    }
}

fn is_redirected(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0 // FILE_ATTRIBUTE_REPARSE_POINT
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn clear_contents(workspace: &Path, expected_root: &Path, dump: &Path) -> Result<()> {
    let filename = dump
        .file_name()
        .with_context(|| format!("Dump file path has no valid filename: {}", dump.display()))?;
    if !workspace.is_absolute()
        || !expected_root.is_absolute()
        || workspace.parent() != Some(expected_root)
        || workspace.file_name() != Some(filename)
    {
        bail!("Invalid workspace cleanup target: {}", workspace.display());
    }
    // Recheck after creation, before any deletion.
    reject_redirected_directory(expected_root)?;
    reject_redirected_directory(workspace)?;
    let root = fs::canonicalize(workspace)
        .with_context(|| format!("Cannot resolve cleanup target: {}", workspace.display()))?;
    let canonical_expected_root = fs::canonicalize(expected_root)
        .with_context(|| format!("Cannot resolve workspace root: {}", expected_root.display()))?;
    if root.parent() != Some(canonical_expected_root.as_path()) {
        bail!(
            "Workspace cleanup target escapes its root directory: {}",
            workspace.display()
        );
    }
    let absolute_dump = path::absolute(dump)
        .with_context(|| format!("Cannot resolve input dump path: {}", dump.display()))?;
    if absolute_dump.starts_with(workspace) {
        bail!(
            "Refusing to clear a workspace containing the input dump: {}",
            dump.display()
        );
    }
    // Check every lexical ancestor: an input path may pass through a workspace link
    // whose target and immediate parent resolve outside the cleanup root.
    for source_parent in absolute_dump.ancestors().skip(1) {
        match fs::canonicalize(source_parent) {
            Ok(parent) if parent.starts_with(&root) => {
                bail!(
                    "Refusing to clear a workspace containing the input dump: {}",
                    dump.display()
                );
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "Cannot resolve input dump parent: {}",
                        source_parent.display()
                    )
                });
            }
        }
    }
    match fs::canonicalize(dump) {
        Ok(source) if source.starts_with(&root) => {
            bail!(
                "Refusing to clear a workspace containing the input dump: {}",
                dump.display()
            );
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error)
                .with_context(|| format!("Cannot resolve input dump path: {}", dump.display()));
        }
    }

    let entries = fs::read_dir(&root)
        .with_context(|| format!("Cannot read workspace for cleanup: {}", workspace.display()))?;
    for entry in entries {
        let entry =
            entry.with_context(|| format!("Cannot list workspace: {}", workspace.display()))?;
        let entry_path = entry.path();
        if entry_path.parent() != Some(root.as_path()) {
            bail!(
                "Cleanup entry escapes its workspace: {}",
                entry_path.display()
            );
        }
        let metadata = fs::symlink_metadata(&entry_path)
            .with_context(|| format!("Cannot inspect cleanup entry: {}", entry_path.display()))?;
        if is_redirected(&metadata) {
            remove_link(&entry_path, &metadata).with_context(|| {
                format!("Cannot remove workspace link: {}", entry_path.display())
            })?;
        } else {
            let resolved_entry = fs::canonicalize(&entry_path).with_context(|| {
                format!("Cannot resolve cleanup entry: {}", entry_path.display())
            })?;
            if !resolved_entry.starts_with(&root) || resolved_entry == root {
                bail!(
                    "Cleanup entry escapes its workspace: {}",
                    entry_path.display()
                );
            }
            if metadata.is_dir() {
                // std removes nested links themselves instead of following them.
                fs::remove_dir_all(&entry_path).with_context(|| {
                    format!(
                        "Cannot remove workspace directory: {}",
                        entry_path.display()
                    )
                })?;
            } else {
                fs::remove_file(&entry_path).with_context(|| {
                    format!("Cannot remove workspace file: {}", entry_path.display())
                })?;
            }
        }
    }
    Ok(())
}

fn remove_link(link: &Path, metadata: &fs::Metadata) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x10 != 0 {
            // FILE_ATTRIBUTE_DIRECTORY
            fs::remove_dir(link)
        } else {
            fs::remove_file(link)
        }
    }
    #[cfg(not(windows))]
    {
        let _ = metadata;
        fs::remove_file(link)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_path_uses_the_system_temporary_directory() {
        let workspace = resolve_path(None, Path::new("heap.hprof")).unwrap();
        let temporary_directory = path::absolute(env::temp_dir()).unwrap();
        assert!(workspace.is_absolute());
        assert_eq!(
            workspace.parent().unwrap().parent().unwrap(),
            temporary_directory
        );
        assert_eq!(workspace.parent().unwrap().file_name().unwrap(), ".jda");
        assert_eq!(workspace.file_name().unwrap(), "heap.hprof");
    }

    #[test]
    fn custom_workspace_creates_parents_and_preserves_the_full_filename() {
        let temporary_directory = tempfile::tempdir().unwrap();
        let base = temporary_directory
            .path()
            .join("analysis files")
            .join("nested");
        let dump = Path::new("source files").join("堆 转储.hprof");
        let workspace = Workspace::prepare(Some(&base), &dump, false).unwrap();
        assert_eq!(workspace.path(), base.join("堆 转储.hprof"));
        assert!(workspace.path().is_dir());
        assert!(workspace.path().is_absolute());
    }

    #[test]
    fn relative_base_is_resolved_without_changing_the_current_directory() {
        let current_directory = env::current_dir().unwrap();
        let workspace =
            resolve_path(Some(Path::new("analysis files")), Path::new("heap.hprof")).unwrap();
        assert_eq!(
            workspace,
            current_directory.join("analysis files").join("heap.hprof")
        );
        assert_eq!(env::current_dir().unwrap(), current_directory);
    }

    #[test]
    fn dot_base_uses_the_startup_directory_directly() {
        let current_directory = env::current_dir().unwrap();
        let workspace = resolve_path(Some(Path::new(".")), Path::new("heap.hprof")).unwrap();
        assert_eq!(workspace, current_directory.join("heap.hprof"));
        assert_eq!(env::current_dir().unwrap(), current_directory);
    }

    #[test]
    fn a_filesystem_root_is_a_valid_custom_root_without_a_parent() {
        let current_directory = env::current_dir().unwrap();
        let filesystem_root = current_directory.ancestors().last().unwrap();
        let workspace = resolve_path(Some(filesystem_root), Path::new("heap.hprof")).unwrap();
        assert!(filesystem_root.parent().is_none());
        assert_eq!(workspace.parent(), Some(filesystem_root));
        assert_eq!(workspace.file_name().unwrap(), "heap.hprof");
    }

    #[test]
    fn cleanup_rejects_a_target_with_the_wrong_root_or_filename() {
        let temporary_directory = tempfile::tempdir().unwrap();
        let base = temporary_directory.path().join("work");
        let dump = Path::new("heap.hprof");
        let workspace = Workspace::prepare(Some(&base), dump, false).unwrap();
        let artifact = workspace.path().join("analysis.bin");
        fs::write(&artifact, b"cache").unwrap();
        assert!(clear_contents(workspace.path(), temporary_directory.path(), dump).is_err());
        assert!(clear_contents(workspace.path(), &base, Path::new("other.hprof")).is_err());
        assert_eq!(fs::read(artifact).unwrap(), b"cache");
    }

    #[test]
    fn preparing_again_preserves_existing_analysis_artifacts() {
        let temporary_directory = tempfile::tempdir().unwrap();
        let dump = Path::new("heap.hprof");
        let first = Workspace::prepare(Some(temporary_directory.path()), dump, false).unwrap();
        let artifact = first.path().join("histogram.bin");
        fs::write(&artifact, b"cached analysis").unwrap();
        let second = Workspace::prepare(Some(temporary_directory.path()), dump, false).unwrap();
        assert_eq!(first.path(), second.path());
        assert_eq!(fs::read(artifact).unwrap(), b"cached analysis");
    }

    #[test]
    fn an_existing_file_at_the_workspace_path_reports_the_path() {
        let temporary_directory = tempfile::tempdir().unwrap();
        let occupied_path = temporary_directory.path().join("heap.hprof");
        fs::write(&occupied_path, b"existing file").unwrap();
        for force in [false, true] {
            for dump in [Path::new("heap.hprof"), occupied_path.as_path()] {
                let error =
                    Workspace::prepare(Some(temporary_directory.path()), dump, force).unwrap_err();
                assert!(
                    error
                        .to_string()
                        .contains("Cannot create analysis workspace")
                );
                assert!(
                    error
                        .to_string()
                        .contains(&occupied_path.display().to_string())
                );
                assert_eq!(fs::read(&occupied_path).unwrap(), b"existing file");
            }
        }
    }

    #[test]
    fn invalid_base_and_missing_filename_are_rejected() {
        assert!(resolve_path(Some(Path::new("")), Path::new("heap.hprof")).is_err());
        for dump in ["", "..", std::path::MAIN_SEPARATOR_STR] {
            assert!(
                resolve_path(None, Path::new(dump)).is_err(),
                "dump: {dump:?}"
            );
        }
    }

    #[test]
    fn force_creates_a_missing_workspace_and_accepts_an_empty_one() {
        let temporary_directory = tempfile::tempdir().unwrap();
        let base = temporary_directory.path().join("new base");
        let first = Workspace::prepare(Some(&base), Path::new("heap.hprof"), true).unwrap();
        assert!(first.path().is_dir());
        let second = Workspace::prepare(Some(&base), Path::new("heap.hprof"), true).unwrap();
        assert_eq!(first.path(), second.path());
        assert!(fs::read_dir(second.path()).unwrap().next().is_none());
    }

    #[test]
    fn force_clears_all_contents_but_preserves_source_and_sibling_workspaces() {
        let temporary_directory = tempfile::tempdir().unwrap();
        let base = temporary_directory.path().join("work");
        let dump = temporary_directory.path().join("heap.hprof");
        fs::write(&dump, b"source dump").unwrap();
        let workspace = Workspace::prepare(Some(&base), &dump, false).unwrap();
        fs::write(workspace.path().join(".hidden-cache"), b"cache").unwrap();
        let nested = workspace.path().join("nested").join("deeper");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("index.bin"), b"index").unwrap();
        let sibling = workspace.path().parent().unwrap().join("other.hprof");
        fs::create_dir(&sibling).unwrap();
        fs::write(sibling.join("analysis.bin"), b"sibling cache").unwrap();
        let legacy_workspace = base.join(".jda").join("heap.hprof");
        fs::create_dir_all(&legacy_workspace).unwrap();
        let legacy_artifact = legacy_workspace.join("legacy.bin");
        fs::write(&legacy_artifact, b"legacy cache").unwrap();
        let outside = base.join("outside.bin");
        fs::write(&outside, b"outside").unwrap();
        let cleared = Workspace::prepare(Some(&base), &dump, true).unwrap();
        assert_eq!(workspace.path(), cleared.path());
        assert!(cleared.path().is_dir());
        assert!(fs::read_dir(cleared.path()).unwrap().next().is_none());
        assert_eq!(fs::read(&dump).unwrap(), b"source dump");
        assert_eq!(
            fs::read(sibling.join("analysis.bin")).unwrap(),
            b"sibling cache"
        );
        assert_eq!(fs::read(outside).unwrap(), b"outside");
        assert_eq!(fs::read(legacy_artifact).unwrap(), b"legacy cache");
    }

    #[test]
    fn force_refuses_to_clear_a_workspace_containing_the_input_dump() {
        let temporary_directory = tempfile::tempdir().unwrap();
        let base = temporary_directory.path();
        let workspace = Workspace::prepare(Some(base), Path::new("heap.hprof"), false).unwrap();
        let dump = workspace.path().join("heap.hprof");
        let artifact = workspace.path().join("analysis.bin");
        fs::write(&dump, b"source dump").unwrap();
        fs::write(&artifact, b"cache").unwrap();
        let error = Workspace::prepare(Some(base), &dump, true).unwrap_err();
        assert!(error.to_string().contains("containing the input dump"));
        assert_eq!(fs::read(dump).unwrap(), b"source dump");
        assert_eq!(fs::read(artifact).unwrap(), b"cache");
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn force_protects_input_paths_that_pass_through_workspace_links() {
        let temporary_directory = tempfile::tempdir().unwrap();
        let base = temporary_directory.path().join("work");
        let external = temporary_directory.path().join("external");
        fs::create_dir(&external).unwrap();
        let source = external.join("heap.hprof");
        fs::write(&source, b"source dump").unwrap();
        let workspace = Workspace::prepare(Some(&base), Path::new("heap.hprof"), false).unwrap();
        let artifact = workspace.path().join("analysis.bin");
        fs::write(&artifact, b"cache").unwrap();
        let nested_link = workspace.path().join("nested-link");
        if !create_directory_link(&external, &nested_link) {
            return;
        }

        #[cfg(windows)]
        let input = base
            .join("HEAP.HPROF")
            .join("nested-link")
            .join("heap.hprof");
        #[cfg(unix)]
        let input = {
            let base_alias = temporary_directory.path().join("work-alias");
            assert!(create_directory_link(&base, &base_alias));
            base_alias
                .join("heap.hprof")
                .join("nested-link")
                .join("heap.hprof")
        };
        assert!(input.is_file());
        let error = Workspace::prepare(Some(&base), &input, true).unwrap_err();
        assert!(error.to_string().contains("containing the input dump"));
        assert_eq!(fs::read(source).unwrap(), b"source dump");
        assert_eq!(fs::read(artifact).unwrap(), b"cache");
        assert!(input.is_file());
        assert!(nested_link.is_dir());
    }

    #[cfg(any(unix, windows))]
    fn create_directory_link(target: &Path, link: &Path) -> bool {
        #[cfg(unix)]
        let result = std::os::unix::fs::symlink(target, link);
        #[cfg(windows)]
        let result = std::os::windows::fs::symlink_dir(target, link);
        match result {
            Ok(()) => true,
            #[cfg(windows)]
            Err(error)
                if error.raw_os_error() == Some(1314)
                    || error.kind() == io::ErrorKind::PermissionDenied =>
            {
                eprintln!(
                    "Skipping directory symlink test: symlink creation permission is unavailable"
                );
                false
            }
            Err(error) => panic!("Cannot create test directory link: {error}"),
        }
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn force_removes_directory_links_without_touching_their_targets() {
        let temporary_directory = tempfile::tempdir().unwrap();
        let base = temporary_directory.path().join("work");
        let external = temporary_directory.path().join("external");
        fs::create_dir(&external).unwrap();
        fs::write(external.join("keep.bin"), b"external").unwrap();
        let workspace = Workspace::prepare(Some(&base), Path::new("heap.hprof"), false).unwrap();
        let link = workspace.path().join("external-link");
        if !create_directory_link(&external, &link) {
            return;
        }
        Workspace::prepare(Some(&base), Path::new("heap.hprof"), true).unwrap();
        assert!(!link.exists());
        assert_eq!(fs::read(external.join("keep.bin")).unwrap(), b"external");
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn force_rejects_linked_workspace_and_effective_roots() {
        let temporary_directory = tempfile::tempdir().unwrap();
        let external = temporary_directory.path().join("external");
        fs::create_dir(&external).unwrap();
        let artifact = external.join("keep.bin");
        fs::write(&artifact, b"external").unwrap();
        let base = temporary_directory.path().join("work");
        fs::create_dir(&base).unwrap();
        let linked_workspace = base.join("heap.hprof");
        if !create_directory_link(&external, &linked_workspace) {
            return;
        }
        let error = Workspace::prepare(Some(&base), Path::new("heap.hprof"), true).unwrap_err();
        assert!(error.to_string().contains("linked workspace directory"));
        assert_eq!(fs::read(&artifact).unwrap(), b"external");
        let linked_root = temporary_directory.path().join("linked-root");
        if !create_directory_link(&external, &linked_root) {
            return;
        }
        let error =
            Workspace::prepare(Some(&linked_root), Path::new("heap.hprof"), true).unwrap_err();
        assert!(error.to_string().contains("linked workspace directory"));
        assert_eq!(fs::read(artifact).unwrap(), b"external");
    }
}

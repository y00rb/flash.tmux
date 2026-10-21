use crate::{
    Result,
    config::Config,
    model::Outcome,
    terminal,
    tmux::{self, Snapshot},
};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
struct Request {
    snapshot: Snapshot,
    config: Config,
    mode: crate::model::InteractionMode,
    capture: Option<crate::syntax::document::CapturedDocument>,
}

pub fn quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

fn lock_dir() -> Result<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME is unset")?);
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".cache"));
    fs::create_dir_all(&cache)?;
    let dir = cache.join("flash-tmux");
    match fs::DirBuilder::new().mode(0o700).create(&dir) {
        Ok(()) => (),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(e) => return Err(e.into()),
    }
    let metadata = fs::symlink_metadata(&dir)?;
    if !metadata.is_dir()
        || metadata.uid() != fs::metadata(&home)?.uid()
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err("flash-tmux cache directory must be a private, user-owned directory".into());
    }
    Ok(dir)
}

pub fn start(pane: &str, client: &str) -> Result<()> {
    start_mode(pane, client, crate::model::InteractionMode::Search)
}

pub fn start_mode(pane: &str, client: &str, mode: crate::model::InteractionMode) -> Result<()> {
    let config = tmux::config_for_pane(pane)?;
    let original = tmux::snapshot(pane, client)?;
    let dir = lock_dir()?;
    let socket = tmux::command(&["display-message", "-p", "-t", pane, "#{socket_path}"])?;
    use std::hash::{Hash, Hasher};
    let mut locks = Vec::new();
    // tmux has one popup slot per client, even when two panes are distinct.
    for identity in [format!("pane:{pane}"), format!("client:{client}")] {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        socket.hash(&mut hash);
        identity.hash(&mut hash);
        let lock_path = dir.join(format!("{:x}.lock", hash.finish()));
        if fs::symlink_metadata(&lock_path).is_ok_and(|m| !m.is_file()) {
            return Err("invalid lock file".into());
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)?;
        lock.try_lock_exclusive()
            .map_err(|_| "a search is already active for this pane or client")?;
        locks.push(lock);
    }
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    socket.hash(&mut hash);
    pane.hash(&mut hash);
    let prefix = format!("search-{:x}-", hash.finish());
    // We hold the pane lock, so only this pane's previous scratch files are stale.
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        if entry.file_name().to_string_lossy().starts_with(&prefix) {
            if entry.file_type()?.is_dir() {
                fs::remove_dir_all(entry.path())?;
            } else {
                fs::remove_file(entry.path())?;
            }
        }
    }
    let scratch = tempfile::Builder::new().prefix(&prefix).tempdir_in(&dir)?;
    let request_path = scratch.path().join("request.json");
    let result_path = scratch.path().join("result.json");
    fs::write(
        &request_path,
        serde_json::to_vec(&Request {
            snapshot: original.clone(),
            config,
            mode,
            capture: if mode == crate::model::InteractionMode::Search {
                None
            } else {
                Some(tmux::syntax_capture(&original)?)
            },
        })?,
    )?;
    let executable = std::env::current_exe()?;
    let shell = format!(
        "exec {} overlay --snapshot {} --result {}",
        quote(&executable.to_string_lossy()),
        quote(&request_path.to_string_lossy()),
        quote(&result_path.to_string_lossy())
    );
    let result = tmux::command(&[
        "display-popup",
        "-B",
        "-E",
        "-c",
        client,
        "-t",
        pane,
        "-x",
        &original.left.to_string(),
        "-y",
        &(original.top + original.viewport.rows.len()).to_string(),
        "-w",
        &original.viewport.width.to_string(),
        "-h",
        &original.viewport.rows.len().to_string(),
        &shell,
    ]);
    let _ = tmux::command(&["display-message", "-c", client, "-d", "1", ""]);
    let error_path = result_path.with_extension("error");
    if error_path.exists() {
        return Err(fs::read_to_string(error_path)?.into());
    }
    result?;
    if result_path.exists() {
        let outcome: Outcome = serde_json::from_slice(&fs::read(&result_path)?)?;
        match outcome {
            Outcome::Selected { point } => tmux::apply_jump(&original, point)?,
            Outcome::RangeSelected { range } => tmux::apply_range(&original, range)?,
            Outcome::Cancelled => {}
        }
    }
    Ok(())
}

pub fn overlay(request_path: &Path, result_path: &Path) -> Result<()> {
    let outcome = overlay_inner(request_path, result_path);
    if let Err(error) = &outcome {
        // Popup stderr is not forwarded by tmux; preserve the cause in our
        // private session directory for the parent launcher to report.
        if let Some(parent) = result_path.parent()
            && let Ok(mut file) = tempfile::NamedTempFile::new_in(parent)
            && file.write_all(error.to_string().as_bytes()).is_ok()
        {
            let _ = file.persist(result_path.with_extension("error"));
        }
    }
    outcome
}

fn overlay_inner(request_path: &Path, result_path: &Path) -> Result<()> {
    let request: Request = serde_json::from_slice(&fs::read(request_path)?)?;
    if request.snapshot.version != 2 {
        return Err("unsupported snapshot format".into());
    }
    let outcome = if request.mode == crate::model::InteractionMode::Search {
        terminal::run(
            &request.snapshot.viewport,
            request.snapshot.cursor,
            &request.snapshot.client,
            &request.config,
        )?
    } else {
        terminal::run_syntax(
            request.capture.as_ref().ok_or("missing syntax capture")?,
            &request.config,
            request.mode,
            &request.snapshot,
        )?
    };
    let mut result =
        tempfile::NamedTempFile::new_in(result_path.parent().ok_or("result path needs a parent")?)?;
    serde_json::to_writer(&mut result, &outcome)?;
    result.flush()?;
    result.persist(result_path)?;
    Ok(())
}

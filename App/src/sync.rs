use crate::config::AppConfig;
use notify::event::{EventKind, ModifyKind, RenameMode};
use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Set to stop a running sync engine (mirrors tray::SHOULD_STOP_TRAY),
/// checked by both the poll thread and the worker thread's receive loop.
pub static STOP_SYNC: AtomicBool = AtomicBool::new(false);

const POLL_INTERVAL: Duration = Duration::from_secs(60);
const POLL_CHECK_STEP: Duration = Duration::from_millis(200);
const DEBOUNCE_WINDOW: Duration = Duration::from_millis(1500);

enum SyncEvent {
    LocalTouched(PathBuf),
    Renamed(PathBuf, PathBuf),
    PollTick,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SyncedFile {
    hash: String,
    version: u64,
    /// The server's stable NoteFile uuid. Lets reconcile() recognize "this
    /// relpath I've never seen is actually a rename of a relpath I already
    /// know" instead of treating it as delete-old + download-new. Empty for
    /// entries written before this field existed; those self-heal the next
    /// time that file is touched (see reconcile()).
    #[serde(default)]
    uuid: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct SyncState {
    files: HashMap<String, SyncedFile>,
}

impl SyncState {
    fn path() -> PathBuf {
        let base = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("syncnotes");
        fs::create_dir_all(&base).ok();
        base.join("sync-state.json")
    }

    fn load() -> Self {
        fs::read_to_string(Self::path())
            .ok()
            .and_then(|data| serde_json::from_str(&data).ok())
            .unwrap_or_default()
    }

    fn save(&self) {
        if let Ok(data) = serde_json::to_string_pretty(self) {
            fs::write(Self::path(), data).ok();
        }
    }
}

#[derive(Debug, Deserialize)]
struct ManifestEntry {
    #[serde(default)]
    uuid: String,
    relpath: String,
    version: u64,
    #[serde(default)]
    deleted: bool,
}

#[derive(Debug, Deserialize)]
struct ManifestResponse {
    files: Vec<ManifestEntry>,
}

#[derive(Debug, Deserialize)]
struct SyncResponse {
    #[serde(default)]
    uuid: String,
    version: u64,
    hash: String,
    #[serde(default)]
    has_pdf: bool,
}


pub struct SyncHandle {
    pub join: thread::JoinHandle<()>,
}

/// Starts the sync engine as a background thread pair: one thread owns all
/// sync state exclusively (no locking needed) and processes events pulled
/// off a channel; the other just ticks a periodic poll trigger. Call once
/// per process start - the config in effect at that moment is used for the
/// lifetime of this engine (a settings change takes effect on next restart).
pub fn spawn(config: AppConfig) -> SyncHandle {
    STOP_SYNC.store(false, Ordering::Relaxed);
    let (tx, rx) = mpsc::channel::<SyncEvent>();

    let poll_tx = tx.clone();
    thread::spawn(move || {
        let mut elapsed = Duration::ZERO;
        loop {
            if STOP_SYNC.load(Ordering::Relaxed) {
                return;
            }
            thread::sleep(POLL_CHECK_STEP);
            elapsed += POLL_CHECK_STEP;
            if elapsed >= POLL_INTERVAL {
                elapsed = Duration::ZERO;
                if poll_tx.send(SyncEvent::PollTick).is_err() {
                    return;
                }
            }
        }
    });

    let join = thread::spawn(move || run(config, tx, rx));
    SyncHandle { join }
}

fn run(config: AppConfig, tx: mpsc::Sender<SyncEvent>, rx: mpsc::Receiver<SyncEvent>) {
    let root = PathBuf::from(&config.rnotes_dir);
    fs::create_dir_all(&root).ok();

    let mut state = SyncState::load();

    let debouncer = new_debouncer(DEBOUNCE_WINDOW, None, move |result: DebounceEventResult| {
        if let Ok(events) = result {
            for event in events {
                // notify-debouncer-full correlates a rename's "from" and "to"
                // halves into one event when it can, so a plain edit-in-place
                // rename shows up as a single clean pair here rather than a
                // Remove+Create - handle that specially so it's treated as a
                // rename (keeping version history) instead of losing it.
                if let EventKind::Modify(ModifyKind::Name(RenameMode::Both)) = event.kind {
                    if event.paths.len() == 2 {
                        let _ = tx.send(SyncEvent::Renamed(
                            event.paths[0].clone(),
                            event.paths[1].clone(),
                        ));
                        continue;
                    }
                }
                for path in &event.paths {
                    let _ = tx.send(SyncEvent::LocalTouched(path.clone()));
                }
            }
        }
    });

    let mut debouncer = debouncer.ok();
    if let Some(d) = debouncer.as_mut() {
        let mode = if config.sync_subdirs {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        let _ = d.watch(&root, mode);
    }

    if let Some(manifest) = fetch_manifest(&config) {
        reconcile(&config, &root, &mut state, &manifest, true);
        state.save();
    }

    loop {
        if STOP_SYNC.load(Ordering::Relaxed) {
            break;
        }
        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(SyncEvent::LocalTouched(path)) => {
                handle_local_touched(&config, &root, &mut state, &path);
                state.save();
            }
            Ok(SyncEvent::Renamed(from, to)) => {
                handle_renamed(&config, &root, &mut state, &from, &to);
                state.save();
            }
            Ok(SyncEvent::PollTick) => {
                if let Some(manifest) = fetch_manifest(&config) {
                    reconcile(&config, &root, &mut state, &manifest, false);
                    state.save();
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    drop(debouncer);
}

fn reconcile(
    config: &AppConfig,
    root: &Path,
    state: &mut SyncState,
    manifest: &[ManifestEntry],
    full_local_walk: bool,
) {
    // Snapshot uuid -> relpath before this pass mutates state.files, so a
    // rename detected partway through the loop below is judged against
    // where things stood at the start, not against edits made moments ago
    // in this same pass.
    let known_by_uuid: HashMap<String, String> = state
        .files
        .iter()
        .filter(|(_, f)| !f.uuid.is_empty())
        .map(|(relpath, f)| (f.uuid.clone(), relpath.clone()))
        .collect();

    for entry in manifest {
        if !config.sync_subdirs && entry.relpath.contains('/') {
            continue;
        }
        let full_path = root.join(&entry.relpath);
        let mut local_exists = full_path.exists();

        if entry.deleted {
            if let Some(known) = state.files.get(&entry.relpath) {
                if local_exists {
                    if let Some(current_hash) = hash_file(&full_path) {
                        if current_hash == known.hash {
                            fs::remove_file(&full_path).ok();
                            state.files.remove(&entry.relpath);
                        }
                        // else: locally dirty since last sync - leave it, it
                        // wins per LWW and will be uploaded, undoing the delete.
                    }
                }
            }
            continue;
        }

        // This relpath is new to us, but its uuid matches a relpath we do
        // know - another device renamed it. Move the local file instead of
        // falling through to "download a brand new copy" below (which would
        // leave the old one behind and lose nothing but still look like a
        // duplicate rather than a rename).
        if !state.files.contains_key(&entry.relpath) && !entry.uuid.is_empty() {
            if let Some(old_relpath) = known_by_uuid.get(&entry.uuid) {
                if old_relpath != &entry.relpath && !local_exists {
                    if let Some(old_synced) = state.files.get(old_relpath).cloned() {
                        let old_full_path = root.join(old_relpath);
                        if old_full_path.exists() {
                            if let Some(parent) = full_path.parent() {
                                fs::create_dir_all(parent).ok();
                            }
                            if fs::rename(&old_full_path, &full_path).is_ok() {
                                state.files.remove(old_relpath);
                                state.files.insert(
                                    entry.relpath.clone(),
                                    SyncedFile {
                                        hash: old_synced.hash,
                                        version: old_synced.version,
                                        uuid: old_synced.uuid,
                                    },
                                );
                                local_exists = true;
                                // Falls through to the match below, which
                                // will notice if entry.version is actually
                                // newer (content also changed elsewhere)
                                // and download the update on top.
                            }
                        }
                    }
                }
            }
        }

        match state.files.get(&entry.relpath).cloned() {
            None => {
                if local_exists {
                    // This device already has a file at this path that the
                    // server also knows about (fresh install, or a stray
                    // file). Upload wins per LWW - the prior server content
                    // is preserved in version history either way.
                    if let Some((version, hash, uuid)) = upload_file_with_pdf_fallback(config, root, &entry.relpath) {
                        state.files.insert(entry.relpath.clone(), SyncedFile { hash, version, uuid });
                    }
                } else if let Some(hash) = download_file(config, root, &entry.relpath, None) {
                    state.files.insert(
                        entry.relpath.clone(),
                        SyncedFile { hash, version: entry.version, uuid: entry.uuid.clone() },
                    );
                }
            }
            Some(known) => {
                if !local_exists {
                    // Known to this device before, missing now - propagate
                    // as a local delete, don't resurrect it.
                    if delete_remote(config, &entry.relpath) {
                        state.files.remove(&entry.relpath);
                    }
                    continue;
                }
                match hash_file(&full_path) {
                    Some(h) if h == known.hash => {
                        if entry.version > known.version {
                            if let Some(new_hash) = download_file(config, root, &entry.relpath, None) {
                                state.files.insert(
                                    entry.relpath.clone(),
                                    SyncedFile { hash: new_hash, version: entry.version, uuid: entry.uuid.clone() },
                                );
                            }
                        } else if known.uuid.is_empty() && !entry.uuid.is_empty() {
                            // Backfill the uuid on an already-synced entry
                            // written before this field existed, so a future
                            // rename of this file is detected immediately
                            // rather than after its next content change.
                            state.files.insert(
                                entry.relpath.clone(),
                                SyncedFile { hash: known.hash, version: known.version, uuid: entry.uuid.clone() },
                            );
                        }
                    }
                    Some(_) => {
                        // Edited locally since last sync - upload, it wins.
                        if let Some((version, hash, uuid)) = upload_file_with_pdf_fallback(config, root, &entry.relpath) {
                            state.files.insert(entry.relpath.clone(), SyncedFile { hash, version, uuid });
                        }
                    }
                    None => {}
                }
            }
        }
    }

    if full_local_walk {
        let mut to_upload = Vec::new();
        walk_local(root, root, config.sync_subdirs, &mut |relpath| {
            if is_rnote(&relpath) && !manifest.iter().any(|e| e.relpath == relpath) {
                to_upload.push(relpath);
            }
        });
        for relpath in to_upload {
            if let Some((version, hash, uuid)) = upload_file_with_pdf_fallback(config, root, &relpath) {
                state.files.insert(relpath, SyncedFile { hash, version, uuid });
            }
        }
    }
}

fn handle_local_touched(config: &AppConfig, root: &Path, state: &mut SyncState, path: &Path) {
    let relpath = match relpath_for(root, path) {
        Some(r) => r,
        None => return,
    };
    if !is_rnote(&relpath) {
        return;
    }
    if !config.sync_subdirs && relpath.contains('/') {
        return;
    }

    if !path.exists() {
        if state.files.contains_key(&relpath) && delete_remote(config, &relpath) {
            state.files.remove(&relpath);
        }
        return;
    }

    let current_hash = match hash_file(path) {
        Some(h) => h,
        None => return,
    };

    if let Some(known) = state.files.get(&relpath) {
        if known.hash == current_hash {
            return;
        }
    }

    if let Some((version, hash, uuid)) = upload_file_with_pdf_fallback(config, root, &relpath) {
        state.files.insert(relpath, SyncedFile { hash, version, uuid });
    }
}

/// Handles a cleanly-matched rename pair from the debouncer. Renames the
/// NoteFile server-side (keeping its version history) instead of letting
/// it fall through to the normal delete+upload path, which would make the
/// history look like it belonged to a deleted file and start a brand new
/// one at version 1.
fn handle_renamed(config: &AppConfig, root: &Path, state: &mut SyncState, from: &Path, to: &Path) {
    let to_relpath = match relpath_for(root, to) {
        Some(r) => r,
        None => return,
    };
    let from_relpath = match relpath_for(root, from) {
        Some(r) => r,
        None => {
            // "from" is outside the watched root (moved in from elsewhere) -
            // nothing to rename server-side, just upload "to" fresh.
            handle_local_touched(config, root, state, to);
            return;
        }
    };

    if !is_rnote(&to_relpath) || (!config.sync_subdirs && to_relpath.contains('/')) {
        // Renamed to something we don't sync (or now out of sync_subdirs
        // scope) - treat the old path as a plain local removal.
        if state.files.contains_key(&from_relpath) && delete_remote(config, &from_relpath) {
            state.files.remove(&from_relpath);
        }
        return;
    }

    let known = match state.files.get(&from_relpath).cloned() {
        // Never synced this path before - nothing to preserve, just upload
        // fresh at the new name.
        None => {
            handle_local_touched(config, root, state, to);
            return;
        }
        Some(known) => known,
    };

    if rename_remote(config, &from_relpath, &to_relpath) {
        state.files.remove(&from_relpath);
        state.files.insert(to_relpath, known);
    } else {
        // Target collided with an existing/deleted note server-side, or the
        // request failed outright - fall back to the safe delete+upload
        // path so nothing is lost, even though history won't carry over.
        if delete_remote(config, &from_relpath) {
            state.files.remove(&from_relpath);
        }
        handle_local_touched(config, root, state, to);
    }
}

fn walk_local(base: &Path, dir: &Path, recursive: bool, visit: &mut dyn FnMut(String)) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if recursive {
                walk_local(base, &path, recursive, visit);
            }
        } else if let Some(relpath) = relpath_for(base, &path) {
            visit(relpath);
        }
    }
}

fn relpath_for(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let s = rel.to_string_lossy().replace('\\', "/");
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn is_rnote(relpath: &str) -> bool {
    relpath.to_lowercase().ends_with(".rnote")
}

fn hash_file(path: &Path) -> Option<String> {
    let mut file = fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect())
}

fn auth_header(config: &AppConfig) -> String {
    format!("Bearer {}", config.access_token)
}

fn fetch_manifest(config: &AppConfig) -> Option<Vec<ManifestEntry>> {
    let url = format!("{}/api/notes/manifest", config.server_url.trim_end_matches('/'));
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(&url)
        .header("Authorization", auth_header(config))
        .send()
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let data: ManifestResponse = resp.json().ok()?;
    Some(data.files)
}

/// Uploads the file, then - if the server didn't already manage to render
/// its own PDF (e.g. rnote-cli isn't installed on that deployment) - tries
/// to render one locally and upload it as a fallback. Best-effort: a
/// missing/failing local rnote-cli just means no PDF, same as today.
fn upload_file_with_pdf_fallback(config: &AppConfig, root: &Path, relpath: &str) -> Option<(u64, String, String)> {
    let (version, hash, has_pdf, uuid) = upload_file(config, root, relpath)?;
    if !has_pdf {
        if let Some(pdf_path) = try_local_pdf(root, relpath) {
            upload_pdf(config, relpath, version, &pdf_path);
            fs::remove_file(&pdf_path).ok();
        }
    }
    Some((version, hash, uuid))
}

fn upload_file(config: &AppConfig, root: &Path, relpath: &str) -> Option<(u64, String, bool, String)> {
    let bytes = fs::read(root.join(relpath)).ok()?;
    let url = format!("{}/api/notes/sync", config.server_url.trim_end_matches('/'));
    let form = reqwest::blocking::multipart::Form::new()
        .text("relpath", relpath.to_string())
        .part(
            "file",
            reqwest::blocking::multipart::Part::bytes(bytes).file_name("upload.rnote"),
        );
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(&url)
        .header("Authorization", auth_header(config))
        .multipart(form)
        .send()
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let data: SyncResponse = resp.json().ok()?;
    Some((data.version, data.hash, data.has_pdf, data.uuid))
}

/// Renders relpath to a PDF using a local rnote-cli, if one happens to be
/// installed on this machine. If the binary isn't found, `Command::output`
/// fails with `NotFound` and this just returns `None` - no separate "is it
/// on PATH" probe needed.
fn try_local_pdf(root: &Path, relpath: &str) -> Option<PathBuf> {
    let rnote_path = root.join(relpath);
    let pdf_path = std::env::temp_dir().join(format!("syncnotes-{}.pdf", uuid_like()));

    let output = std::process::Command::new("rnote-cli")
        .arg("export")
        .arg("doc")
        .arg(&rnote_path)
        .arg("--output-file")
        .arg(&pdf_path)
        .arg("--on-conflict")
        .arg("overwrite")
        .output()
        .ok()?;

    if output.status.success() && pdf_path.exists() {
        Some(pdf_path)
    } else {
        None
    }
}

fn uuid_like() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}-{:x}", nanos, std::process::id())
}

fn upload_pdf(config: &AppConfig, relpath: &str, version: u64, pdf_path: &Path) -> bool {
    let bytes = match fs::read(pdf_path) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let url = format!("{}/api/notes/pdf", config.server_url.trim_end_matches('/'));
    let form = reqwest::blocking::multipart::Form::new()
        .text("relpath", relpath.to_string())
        .text("version", version.to_string())
        .part(
            "pdf",
            reqwest::blocking::multipart::Part::bytes(bytes).file_name("upload.pdf"),
        );
    let client = reqwest::blocking::Client::new();
    client
        .post(&url)
        .header("Authorization", auth_header(config))
        .multipart(form)
        .send()
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

fn download_file(config: &AppConfig, root: &Path, relpath: &str, version: Option<u64>) -> Option<String> {
    let url = format!("{}/api/notes/download", config.server_url.trim_end_matches('/'));
    let client = reqwest::blocking::Client::new();
    let mut req = client
        .get(&url)
        .header("Authorization", auth_header(config))
        .query(&[("relpath", relpath)]);
    if let Some(v) = version {
        req = req.query(&[("version", v.to_string())]);
    }
    let resp = req.send().ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let bytes = resp.bytes().ok()?;

    let full_path = root.join(relpath);
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).ok()?;
    }
    fs::write(&full_path, &bytes).ok()?;

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Some(hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect())
}

fn rename_remote(config: &AppConfig, from_relpath: &str, to_relpath: &str) -> bool {
    let url = format!("{}/api/notes/rename", config.server_url.trim_end_matches('/'));
    let client = reqwest::blocking::Client::new();
    client
        .post(&url)
        .header("Authorization", auth_header(config))
        .form(&[("from_relpath", from_relpath), ("to_relpath", to_relpath)])
        .send()
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

fn delete_remote(config: &AppConfig, relpath: &str) -> bool {
    let url = format!("{}/api/notes/delete", config.server_url.trim_end_matches('/'));
    let client = reqwest::blocking::Client::new();
    client
        .post(&url)
        .header("Authorization", auth_header(config))
        .form(&[("relpath", relpath)])
        .send()
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

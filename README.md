# SyncNotes

![License](https://img.shields.io/badge/license-Apache--2.0-blue) ![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20Windows-lightgrey)

Self-hosted sync for your [Rnote](https://github.com/flxzt/rnote) `.rnote` files — a small Rust desktop app that watches a folder and keeps it in sync with your own private Flask server, plus a web dashboard to browse notes, view version history, and read them as PDFs from any browser.

**You run your own server.** There's no shared/hosted instance — SyncNotes is software you deploy for yourself (or your household/team), not a service you sign up for.

---

## Contents

- [Features](#features)
- [Quick Install](#quick-install)
- [Server](#server)
- [Desktop App](#desktop-app)
- [Updating](#updating)
- [Uninstalling](#uninstalling)
- [Building from source](#building-from-source)
- [Configuration reference](#configuration-reference)
- [How it works](#how-it-works)
- [License](#license)

---

## Features

- **Automatic two-way sync** — edit, rename, move, or delete a `.rnote` file locally and it's reflected on the server (and every other synced device) within a minute, or instantly on save via filesystem watching.
- **Full version history** — every upload is kept forever; browse and download any previous version from the web dashboard.
- **Rename-aware** — renaming or moving a file is tracked as a rename, not a delete + new file, so its history follows it.
- **PDF rendering** — the server renders a PDF preview of each note automatically (via `rnote-cli`), viewable right in the browser with pan/zoom, no Rnote install required to check a note from your phone or a friend's PC. If the server can't render one, the desktop app falls back to rendering it locally.
- **Conflict handling** — last-write-wins by version, so a stray offline edit never silently overwrites newer work.
- **Trash** — deleted notes go to a recoverable trash instead of disappearing immediately.
- **Multi-device, multi-user** — OAuth-style device-code pairing (like signing into a TV app), each device shows up under its own name; admins can manage users, reset passwords, and browse (read-only) other users' notes.
- **Lightweight desktop app** — a small native Rust/egui app with a tray icon; idles at effectively 0% CPU between syncs.

## Quick Install

Clone the repo and run the installer for your OS. It asks whether you want the **Desktop App**, the **Server**, or both, and installs everything needed.

**Linux**
```bash
git clone https://github.com/akuras22/SyncNotes.git
cd SyncNotes
./install-linux.sh
```

**Windows** (run from a regular Command Prompt or PowerShell)
```batch
git clone https://github.com/akuras22/SyncNotes.git
cd SyncNotes
install-windows.bat
```

The installer detects your Linux distro's package manager automatically, offers to download a prebuilt app binary (falling back to building from source if none is available for your architecture), and — for the server — generates a random secret key and asks a couple of quick questions before starting Docker Compose.

## Server

You need one server, reachable by every device you want to sync. Requires **Docker** and **Docker Compose**.

```bash
cd Server
cp .env.example .env
# edit .env - at minimum change ADMIN_PASSWORD and SECRET_KEY
docker compose up -d --build
```

The server listens on **port 2394**. Open `http://<your-server>:2394` and log in with the admin account from `.env` (change the password immediately from Settings).

By default it uses SQLite (zero extra setup — just a file under `Server/instance/`). To use MySQL instead, set `DATABASE_URL` (or the individual `DB_*` variables) in `.env` — see [Configuration reference](#configuration-reference). The installer script asks which one you want and fills this in for you.

Put the server behind a reverse proxy (nginx, Caddy, Traefik, ...) for TLS/HTTPS if it's reachable from outside your LAN — SyncNotes itself only speaks plain HTTP.

### Server dependencies

The Docker image builds [`rnote-cli`](https://github.com/flxzt/rnote) from source in a multi-stage build (it isn't published as a binary anywhere) so the server can render PDF previews. This makes the first `docker compose up -d --build` take a few minutes; later rebuilds are cached. If your server is slower than your workstation, you can build the image elsewhere and push it to a registry (the compose file already references an `image:` tag alongside `build: .`, so `docker compose pull` on the server works once you've pushed one).

## Desktop App

The desktop app watches a local folder and keeps it synced with your server.

**First run** launches a setup wizard:
1. Enter your server's URL.
2. Authorize the device via your browser (device-code flow — same idea as pairing a smart TV).
3. Pick the folder containing your `.rnote` files.
4. Choose whether to start automatically on login and whether to sync subfolders.

After setup, the app runs in the background with a tray icon. Run it again any time to reopen Settings (change server, folder, or sync options) or disconnect the device.

On first connect, it downloads everything already on the server into your chosen folder; after that, local changes upload automatically and remote changes download automatically (checked on save and once a minute).

## Updating

Re-run the installer for your OS — it detects an existing install and offers to update it (server: rebuilds the Docker image; app: re-downloads/rebuilds the binary).

## Uninstalling

**Linux:** `./uninstall-linux.sh`
**Windows:** `uninstall-windows.bat`

Both scripts ask before touching anything destructive. They remove the installed app binary, desktop entry/icon, and app config; for the server they stop the Docker containers and separately ask whether to also delete the database, uploaded files, and `.env`. Either way, the **local folder the desktop app watches is never touched** — only its own config and the server's copies are ever up for removal.

## Building from source

If you'd rather skip the installer:

**Desktop app** — needs [Rust](https://rustup.rs/), and on Linux: GTK 3, WebKit2GTK 4.1, librsvg, and an app-indicator library (`libappindicator3` or `libayatana-appindicator3`) for the tray icon.
```bash
cd App
cargo build --release
./target/release/syncnotes-app
```

**Server** — needs Docker (recommended, handles the `rnote-cli` build for you), or Python 3.12+ if you want to run it directly:
```bash
cd Server
docker compose up -d --build
# — or, without Docker (PDF rendering will be unavailable unless rnote-cli is on your PATH) —
python3 -m venv venv && venv/bin/pip install -r requirements.txt
cp .env.example .env  # edit it first
venv/bin/python app.py
```

## Configuration reference

All server settings live in `Server/.env` (copy `Server/.env.example` to start).

| Variable | Default | Notes |
|---|---|---|
| `SECRET_KEY` | — | Flask session signing key. **Set this to a long random value.** |
| `DATABASE_URL` | *(unset)* | Full SQLAlchemy URL; overrides the `DB_*` variables below. Use `sqlite:///instance/syncnotes.db` for SQLite. |
| `DB_HOST` / `DB_PORT` / `DB_NAME` / `DB_USER` / `DB_PASSWORD` | `localhost` / `3306` / `syncnotes` / `syncnotes` / — | Used to build a MySQL connection when `DATABASE_URL` isn't set. |
| `ADMIN_USERNAME` / `ADMIN_EMAIL` / `ADMIN_PASSWORD` | `admin` / `admin@localhost` / `admin123` | Only used to create the first admin account on an empty database. **Change the password before/immediately after first boot** — the default is public since this is open-source. |
| `PREFERRED_URL_SCHEME` | `http` | Set to `https` if the server sits behind a TLS-terminating reverse proxy, so generated links use `https://`. |

## How it works

- **Server**: Flask + SQLAlchemy (MySQL or SQLite), Flask-Login for sessions. Notes are versioned — every upload creates a new `NoteVersion` row rather than overwriting content, so history is never lost. PDF previews are rendered with `rnote-cli`, compiled from source in the Docker image.
- **Desktop app**: Rust + [egui](https://github.com/emilk/egui), no async runtime — a single background worker thread owns all sync state, fed by a debounced filesystem watcher and a periodic poll timer. Conflicts resolve last-write-wins by version number (not wall-clock time, to avoid clock-skew issues). Renames are detected across devices via a stable per-note ID in the sync manifest, so they move the local file instead of re-downloading it as a new one.
- **Auth**: OAuth-style device-code flow for pairing desktop clients (no password ever touches the app), plain session cookies for the web dashboard.

## License

[Apache License 2.0](LICENSE).

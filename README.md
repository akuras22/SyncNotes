# SyncNotes

Sync your Rnote files to your private server.

---

## Quick Install

Download or clone the repo, then run the installer for your OS.

### Linux

```bash
git clone https://github.com/akuras22/SyncNotes.git
cd SyncNotes
./install-linux.sh
```

The installer detects your distro and installs all needed dependencies automatically. You can choose between **Desktop App** or **Server**.

### Windows

```batch
git clone https://github.com/akuras22/SyncNotes.git
cd SyncNotes
.\install-windows.bat
```

Choose **Desktop App** (builds with Rust) or **Server** (requires Docker Desktop).

---

## Server Setup

The server runs on port **2394**. You can deploy it anywhere Docker runs.

### Linux Server

```bash
cd Server
cp .env.example .env
# Edit .env with your settings
docker compose up -d
```

The install script (`install-linux.sh` → **Server**) does this for you and generates secure passwords.

### Windows Server

```batch
cd Server
copy .env.example .env
REM Edit .env with your settings
docker compose up -d
```

Requires Docker Desktop for Windows.

---

## Desktop App

Syncs `.rnote` files from a local folder to your SyncNotes server.

### First Run

```bash
# Just run it — a setup wizard will guide you
syncnotes
```

The wizard will ask for:
1. Your server URL
2. Login via browser (device authorization flow)
3. Your Rnotes folder
4. Auto-start settings

### Settings

Run the app again to open the settings window, where you can change server, folder, or auto-start.

### Uninstall

**Linux:** `./uninstall-linux.sh`
**Windows:** `.\uninstall-windows.bat`

---

## Requirements

- **Linux:** GTK 3, WebKit2GTK 4.1, librsvg (installed automatically by the installer)
- **Windows:** Nothing extra — WebView2 is built into Windows 10+

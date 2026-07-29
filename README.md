# SyncNotes

Sync your Rnote files to your private server. A desktop app + web server combo.

---

## Quick Install

### Linux

```bash
# Clone the repo
git clone https://github.com/akuras22/SyncNotes.git
cd SyncNotes

# Run the installer
./install-linux.sh
```

Choose **Server** (Docker) or **Desktop App** (Rust). The script detects your distro and installs system dependencies automatically.

### Windows

```batch
git clone https://github.com/akuras22/SyncNotes.git
cd SyncNotes
.\install-windows.bat
```

Choose **Server** (Docker Desktop required) or **Desktop App** (builds with Rust).

### macOS

```bash
git clone https://github.com/akuras22/SyncNotes.git
cd SyncNotes

# Install system dependencies
brew install gtk+3 webkit2gtk librsvg

# Build and install the app
cd App
cargo build --release
cp target/release/syncnotes-app /usr/local/bin/syncnotes
```

---

## Server Setup (Docker)

The server runs on port **2394** behind an nginx reverse proxy with Let's Encrypt at [notes.huebler.tech](https://notes.huebler.tech).

### Manual start

```bash
cd Server
cp .env.example .env
# Edit .env with your settings
docker compose up -d
```

### First run

The install script generates a random `SECRET_KEY`, prompts for admin credentials, and lets you choose between **SQLite** (simple) or **MySQL** (advanced).

---

## Desktop App (Rust / egui)

Syncs `.rnote` files from a local folder to the SyncNotes server.

### Requirements

- **Linux:** `libgtk-3-dev`, `libwebkit2gtk-4.1-dev`, `librsvg2-dev`
- **macOS:** `gtk+3`, `webkit2gtk`, `librsvg`
- **Windows:** Nothing extra (WebView2 is built into Windows 10+)

### Usage

```bash
# First run — setup wizard
syncnotes

# Settings window
syncnotes --settings
```

The setup wizard will:
1. Ask for your server URL
2. Open your browser for OAuth login
3. Ask for your Rnotes directory
4. Configure auto-start

### Uninstall

**Linux:**
```bash
./uninstall-linux.sh
```

**Windows:**
```batch
.\uninstall-windows.bat
```

---

## Login Flow

The app uses a **device authorization flow** (like Google/GitHub):

1. Click **Login with Browser** in the app
2. Your browser opens to the server's login page
3. Sign in and click **Allow** to authorize the app
4. The app receives the token automatically

Alternatively, use **Device Code** (smaller button below) for manual entry.

---

## Project Structure

```
SyncNotes/
├── App/                  # Desktop app (Rust + egui)
│   ├── src/
│   │   ├── main.rs       # Entry point
│   │   ├── setup.rs      # First-run wizard
│   │   ├── settings.rs   # Settings window
│   │   ├── auth.rs       # Device code auth flow
│   │   ├── config.rs     # Config management
│   │   └── theme.rs      # Dark theme
│   └── Cargo.toml
├── Server/               # Web server (Python + Flask)
│   ├── app.py            # Main app
│   ├── web.py            # Web routes
│   ├── api.py            # REST API
│   ├── models.py         # Database models
│   ├── templates/        # Jinja2 templates
│   ├── static/           # CSS
│   └── docker-compose.yml
├── install-linux.sh      # Linux installer
├── install-windows.bat   # Windows installer
├── uninstall-linux.sh    # Linux uninstaller
└── uninstall-windows.bat # Windows uninstaller
```

## Development

```bash
cd App
cargo run                 # Run the desktop app
cargo run -- --settings   # Open settings

cd Server
python app.py             # Run the server directly
```

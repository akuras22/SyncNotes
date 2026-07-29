#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${BLUE}[INFO]${NC} $*"; }
ok()    { echo -e "${GREEN}[OK]${NC} $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
err()   { echo -e "${RED}[ERR]${NC} $*"; }

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"

cleanup() { exit; }
trap cleanup INT TERM

select_option() {
    local prompt="$1" opt1="$2" opt2="$3"
    echo ""
    echo -e "${BLUE}$prompt${NC}"
    echo "  1) $opt1"
    echo "  2) $opt2"
    echo ""
    while true; do
        read -rp "Select [1/2]: " choice
        case "$choice" in
            1) return 0 ;;
            2) return 1 ;;
            *) warn "Please enter 1 or 2." ;;
        esac
    done
}

# ── Distro detection ───────────────────────────────────────────────────────

detect_distro() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        echo "$ID"
    else
        echo "unknown"
    fi
}

PKG_MANAGER=""
PKG_UPDATE=""
PKG_INSTALL=""
PKGS_APP=(libgtk-3-dev libwebkit2gtk-4.1-dev librsvg2-dev)

setup_pkg_manager() {
    local distro
    distro=$(detect_distro)

    case "$distro" in
        debian|ubuntu|linuxmint|pop|elementary|zorin|raspbian)
            PKG_MANAGER="apt"
            PKG_UPDATE="sudo apt-get update -qq"
            PKG_INSTALL="sudo apt-get install -y -qq"
            PKGS_APP=(libgtk-3-dev libwebkit2gtk-4.1-dev librsvg2-dev)
            ;;
        arch|manjaro|endeavouros|arco|archarm|cachyos)
            PKG_MANAGER="pacman"
            PKG_UPDATE="sudo pacman -Sy --noconfirm"
            PKG_INSTALL="sudo pacman -S --noconfirm"
            PKGS_APP=(gtk3 webkit2gtk-4.1 librsvg)
            ;;
        fedora)
            PKG_MANAGER="dnf"
            PKG_UPDATE="sudo dnf check-update -q || true"
            PKG_INSTALL="sudo dnf install -y"
            PKGS_APP=(gtk3-devel webkit2gtk4.1-devel librsvg2-devel)
            ;;
        rhel|centos|rocky|almalinux)
            PKG_MANAGER="dnf"
            PKG_UPDATE="sudo dnf check-update -q || true"
            PKG_INSTALL="sudo dnf install -y"
            PKGS_APP=(gtk3-devel webkit2gtk4.1-devel librsvg2-devel)
            ;;
        opensuse*|suse)
            PKG_MANAGER="zypper"
            PKG_UPDATE="sudo zypper refresh"
            PKG_INSTALL="sudo zypper install -y"
            PKGS_APP=(gtk3-devel webkit2gtk4-devel librsvg-devel)
            ;;
        void)
            PKG_MANAGER="xbps"
            PKG_UPDATE="sudo xbps-install -S"
            PKG_INSTALL="sudo xbps-install -y"
            PKGS_APP=(gtk3-devel webkit2gtk-devel librsvg-devel)
            ;;
        alpine)
            PKG_MANAGER="apk"
            PKG_UPDATE="sudo apk update"
            PKG_INSTALL="sudo apk add"
            PKGS_APP=(gtk3-dev webkit2gtk-dev librsvg-dev)
            ;;
        solus)
            PKG_MANAGER="eopkg"
            PKG_UPDATE="sudo eopkg update-repo"
            PKG_INSTALL="sudo eopkg install"
            PKGS_APP=(libgtk-3-devel libwebkit2gtk-4.1-devel librsvg-devel)
            ;;
        *)
            PKG_MANAGER="unknown"
            ;;
    esac
}

install_packages() {
    local pkgs=("$@")
    case "$PKG_MANAGER" in
        apt|pacman|dnf|zypper|xbps|apk|eopkg)
            info "Installing: ${pkgs[*]}"
            $PKG_INSTALL "${pkgs[@]}"
            ;;
        *)
            warn "Unknown package manager. Please install these manually:"
            for p in "${pkgs[@]}"; do echo "  - $p"; done
            echo ""
            if ! select_option "Continue anyway?" "Yes" "Abort"; then
                exit 1
            fi
            ;;
    esac
}

# ── Server Installation ────────────────────────────────────────────────────

install_server() {
    echo ""
    info "Starting server installation..."

    if ! command -v docker &>/dev/null; then
        err "Docker is not installed."
        echo "  Install Docker first: https://docs.docker.com/engine/install/"
        exit 1
    fi

    if ! docker compose version &>/dev/null 2>&1 && ! docker-compose --version &>/dev/null 2>&1; then
        err "Docker Compose is not installed."
        echo "  Install it: https://docs.docker.com/compose/install/"
        exit 1
    fi

    if [ ! -f "$ROOT_DIR/Server/.env" ]; then
        info "Creating Server/.env from .env.example..."
        cp "$ROOT_DIR/Server/.env.example" "$ROOT_DIR/Server/.env"

        SECRET=$(python3 -c "import secrets; print(secrets.token_hex(32))" 2>/dev/null || openssl rand -hex 32 2>/dev/null || echo "change-me-to-a-random-key")
        sed -i "s/generate-a-random-key-here/$SECRET/" "$ROOT_DIR/Server/.env"

        echo ""
        echo -e "${YELLOW}── Server Configuration ──${NC}"
        read -rp "Admin username [admin]: " ADMIN_USER
        ADMIN_USER=${ADMIN_USER:-admin}
        read -rp "Admin email [admin@localhost]: " ADMIN_EMAIL
        ADMIN_EMAIL=${ADMIN_EMAIL:-admin@localhost}
        read -rsp "Admin password [admin123]: " ADMIN_PASS
        echo ""
        ADMIN_PASS=${ADMIN_PASS:-admin123}

        sed -i "s/ADMIN_USERNAME=admin/ADMIN_USERNAME=$ADMIN_USER/" "$ROOT_DIR/Server/.env"
        sed -i "s/ADMIN_EMAIL=admin@localhost/ADMIN_EMAIL=$ADMIN_EMAIL/" "$ROOT_DIR/Server/.env"
        sed -i "s/ADMIN_PASSWORD=admin123/ADMIN_PASSWORD=$ADMIN_PASS/" "$ROOT_DIR/Server/.env"

        echo ""
        if select_option "Use SQLite (simple) or MySQL?" "SQLite" "MySQL"; then
            sed -i "s|DATABASE_URL=|# DATABASE_URL=|" "$ROOT_DIR/Server/.env"
            sed -i "s|DB_HOST=|# DB_HOST=|" "$ROOT_DIR/Server/.env"
            echo "DATABASE_URL=sqlite:///instance/syncnotes.db" >> "$ROOT_DIR/Server/.env"
            ok "Using SQLite."
        else
            warn "MySQL setup requires a running MySQL instance."
            read -rp "DB host [localhost]: " DB_HOST
            DB_HOST=${DB_HOST:-localhost}
            read -rp "DB port [3306]: " DB_PORT
            DB_PORT=${DB_PORT:-3306}
            read -rp "DB name [syncnotes]: " DB_NAME
            DB_NAME=${DB_NAME:-syncnotes}
            read -rp "DB user [syncnotes]: " DB_USER
            DB_USER=${DB_USER:-syncnotes}
            read -rsp "DB password: " DB_PASS
            echo ""
            sed -i "s/DB_HOST=localhost/DB_HOST=$DB_HOST/" "$ROOT_DIR/Server/.env"
            sed -i "s/DB_PORT=3306/DB_PORT=$DB_PORT/" "$ROOT_DIR/Server/.env"
            sed -i "s/DB_NAME=syncnotes/DB_NAME=$DB_NAME/" "$ROOT_DIR/Server/.env"
            sed -i "s/DB_USER=syncnotes/DB_USER=$DB_USER/" "$ROOT_DIR/Server/.env"
            sed -i "s/DB_PASSWORD=your-db-password-here/DB_PASSWORD=$DB_PASS/" "$ROOT_DIR/Server/.env"
            ok "MySQL configured."
        fi

        ok ".env created and configured."
    else
        info "Server/.env already exists, keeping it."
    fi

    mkdir -p "$ROOT_DIR/Server/instance"
    mkdir -p "$ROOT_DIR/Server/uploads"

    echo ""
    info "Starting server via Docker Compose..."
    cd "$ROOT_DIR/Server"

    if docker compose version &>/dev/null 2>&1; then
        docker compose up -d
    else
        docker-compose up -d
    fi

    echo ""
    ok "Server is running!"
    echo ""
    echo "  Access it at:  http://localhost:2394"
    echo "  Admin login:   $(grep ADMIN_USERNAME "$ROOT_DIR/Server/.env" | cut -d= -f2) / $(grep ADMIN_PASSWORD "$ROOT_DIR/Server/.env" | cut -d= -f2)"
    echo ""
    echo "  To stop:       cd Server && docker compose down"
    echo "  To view logs:  cd Server && docker compose logs -f"
    echo ""
}

# ── App Installation ───────────────────────────────────────────────────────

install_app() {
    echo ""
    info "Starting desktop app installation..."

    if ! command -v cargo &>/dev/null; then
        info "Rust is not installed. Installing via rustup..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
        ok "Rust installed."
    else
        ok "Rust is already installed ($(cargo --version))."
    fi

    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        setup_pkg_manager
        info "Checking system dependencies for the GUI app..."
        MISSING=""
        for pkg in "${PKGS_APP[@]}"; do
            ok "Will install: $pkg"
            if ! dpkg -s "$pkg" &>/dev/null 2>&1 && ! pacman -Qi "$pkg" &>/dev/null 2>&1 && ! rpm -q "$pkg" &>/dev/null 2>&1; then
                MISSING="$MISSING $pkg"
            fi
        done 2>/dev/null || true

        if [ -n "$MISSING" ]; then
            warn "Missing system dependencies: $MISSING"
            if select_option "Install missing packages?" "Yes" "Skip (may fail)"; then
                $PKG_UPDATE
                IFS=" " read -ra PKG_ARRAY <<< "$MISSING"
                install_packages "${PKG_ARRAY[@]}"
                ok "Dependencies installed."
            fi
        else
            ok "All system dependencies are present."
        fi
    fi

    echo ""
    info "Building SyncNotes app (this may take a few minutes)..."
    cd "$ROOT_DIR/App"
    cargo clean --quiet 2>/dev/null
    cargo build --release

    BINARY="$ROOT_DIR/App/target/release/syncnotes-app"
    if [ ! -f "$BINARY" ]; then
        err "Build failed — binary not found at $BINARY"
        exit 1
    fi

    echo ""
    info "Installing binary..."

    INSTALL_DIR="/usr/local/bin"
    if [ ! -w "$INSTALL_DIR" ]; then
        INSTALL_DIR="$HOME/.local/bin"
        mkdir -p "$INSTALL_DIR"
    fi

    cp "$BINARY" "$INSTALL_DIR/syncnotes"
    ok "Installed to $INSTALL_DIR/syncnotes"

    # Install logo
    cp "$ROOT_DIR/logo.png" "$INSTALL_DIR/logo.png"

    # Create Desktop Entry
    info "Creating Desktop entry..."
    DESKTOP_DIR="$HOME/.local/share/applications"
    mkdir -p "$DESKTOP_DIR"
    cat > "$DESKTOP_DIR/syncnotes.desktop" <<EOF
[Desktop Entry]
Name=SyncNotes
Comment=Sync your Rnotes to SyncNotes server
Exec=$INSTALL_DIR/syncnotes
Terminal=false
Type=Application
Categories=Utility;
Icon=$INSTALL_DIR/logo.png
EOF
    ok "Desktop entry created at $DESKTOP_DIR/syncnotes.desktop"

    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        SHELL_CONFIG="$HOME/.$(basename "$SHELL")rc"
        if [ -f "$SHELL_CONFIG" ] || [ -f "$HOME/.profile" ]; then
            TARGET="${SHELL_CONFIG:-$HOME/.profile}"
            if ! grep -q "export PATH=\"\$PATH:$INSTALL_DIR\"" "$TARGET" 2>/dev/null; then
                echo "" >> "$TARGET"
                echo "export PATH=\"\$PATH:$INSTALL_DIR\"" >> "$TARGET"
                ok "Added $INSTALL_DIR to PATH in $TARGET"
            fi
        fi
    fi

    echo ""
    ok "SyncNotes app installed successfully!"
    echo ""
  echo "  Run it:        syncnotes"
  echo ""
    echo "  The app will guide you through setup on first run."
    echo "  It will connect to https://notes.huebler.tech by default."
    echo ""
}

# ── Welcome ────────────────────────────────────────────────────────────────

echo ""
echo -e "${BLUE}══════════════════════════════════════${NC}"
echo -e "${BLUE}       SyncNotes Installer${NC}"
echo -e "${BLUE}══════════════════════════════════════${NC}"

if select_option "What would you like to install?" \
    "Server (Docker)" "Desktop App (Rust)"; then
    install_server
else
    install_app
fi

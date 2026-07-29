#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'; GREEN='\033[0;32m'; BLUE='\033[0;34m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'; NC='\033[0m'

info()  { echo -e "  ${BLUE}→${NC} $*"; }
ok()    { echo -e "  ${GREEN}✔${NC} $*"; }
warn()  { echo -e "  ${YELLOW}⚠${NC} $*"; }
err()   { echo -e "  ${RED}✘${NC} $*"; }
header(){ echo -e "\n  ${CYAN}── $* ──${NC}"; }

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
cleanup() { exit; }
trap cleanup INT TERM

select_option() {
    local prompt="$1" opt1="$2" opt2="$3"
    local selected=0
    echo ""
    echo -e "  ${BLUE}${prompt}${NC}"
    while true; do
        if [ $selected -eq 0 ]; then
            echo -e "    ${GREEN}▸${NC} $opt1"
            echo -e "      $opt2"
        else
            echo -e "      $opt1"
            echo -e "    ${GREEN}▸${NC} $opt2"
        fi
        IFS= read -rsn 1 key
        if [[ $key == $'\033' ]]; then
            read -rsn 2 -t 0.01 key
            case "$key" in
                '[A') selected=0 ;;
                '[B') selected=1 ;;
            esac
        elif [[ $key == $'\n' ]] || [[ $key == $'\r' ]]; then
            echo ""; return $selected
        elif [[ $key == '1' ]]; then
            echo ""; return 0
        elif [[ $key == '2' ]]; then
            echo ""; return 1
        fi
        echo -en "\033[2A\033[J"
    done
}

confirm() {
    local msg="$1"
    echo ""
    warn "$msg"
    read -rp "  Continue? [y/N]: " yn
    case "$yn" in y|Y|yes|Yes) return 0 ;; *) return 1 ;; esac
}

# ── Distro detection ───────────────────────────────────────────────────────

detect_distro() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release; echo "$ID"
    else echo "unknown"; fi
}

PKG_MANAGER=""; PKG_UPDATE=""; PKG_INSTALL=""
PKGS_APP=(libgtk-3-dev libwebkit2gtk-4.1-dev librsvg2-dev)

setup_pkg_manager() {
    local distro
    distro=$(detect_distro)
    case "$distro" in
        debian|ubuntu|linuxmint|pop|elementary|zorin|raspbian)
            PKG_MANAGER="apt"; PKG_UPDATE="sudo apt-get update -qq"
            PKG_INSTALL="sudo apt-get install -y -qq"
            PKGS_APP=(libgtk-3-dev libwebkit2gtk-4.1-dev librsvg2-dev) ;;
        arch|manjaro|endeavouros|arco|archarm|cachyos)
            PKG_MANAGER="pacman"; PKG_UPDATE="sudo pacman -Sy --noconfirm"
            PKG_INSTALL="sudo pacman -S --noconfirm"
            PKGS_APP=(gtk3 webkit2gtk-4.1 librsvg) ;;
        fedora)
            PKG_MANAGER="dnf"; PKG_UPDATE="sudo dnf check-update -q || true"
            PKG_INSTALL="sudo dnf install -y"
            PKGS_APP=(gtk3-devel webkit2gtk4.1-devel librsvg2-devel) ;;
        rhel|centos|rocky|almalinux)
            PKG_MANAGER="dnf"; PKG_UPDATE="sudo dnf check-update -q || true"
            PKG_INSTALL="sudo dnf install -y"
            PKGS_APP=(gtk3-devel webkit2gtk4.1-devel librsvg2-devel) ;;
        opensuse*|suse)
            PKG_MANAGER="zypper"; PKG_UPDATE="sudo zypper refresh"
            PKG_INSTALL="sudo zypper install -y"
            PKGS_APP=(gtk3-devel webkit2gtk4-devel librsvg-devel) ;;
        void)
            PKG_MANAGER="xbps"; PKG_UPDATE="sudo xbps-install -S"
            PKG_INSTALL="sudo xbps-install -y"
            PKGS_APP=(gtk3-devel webkit2gtk-devel librsvg-devel) ;;
        alpine)
            PKG_MANAGER="apk"; PKG_UPDATE="sudo apk update"
            PKG_INSTALL="sudo apk add"
            PKGS_APP=(gtk3-dev webkit2gtk-dev librsvg-dev) ;;
        solus)
            PKG_MANAGER="eopkg"; PKG_UPDATE="sudo eopkg update-repo"
            PKG_INSTALL="sudo eopkg install"
            PKGS_APP=(libgtk-3-devel libwebkit2gtk-4.1-devel librsvg-devel) ;;
        *)
            PKG_MANAGER="unknown" ;;
    esac
}

install_packages() {
    local pkgs=("$@")
    case "$PKG_MANAGER" in
        apt|pacman|dnf|zypper|xbps|apk|eopkg)
            info "Installing: ${pkgs[*]}"
            $PKG_INSTALL "${pkgs[@]}" ;;
        *)
            warn "Unknown package manager. Install these manually:"
            for p in "${pkgs[@]}"; do echo "    - $p"; done
            echo ""
            if ! select_option "Continue anyway?" "Yes" "Abort"; then exit 1; fi ;;
    esac
}

# ── Server Installation ────────────────────────────────────────────────────

install_server() {
    echo ""
    header "Server Installation"

    if ! command -v docker &>/dev/null; then
        err "Docker is not installed."
        echo "    Install: https://docs.docker.com/engine/install/"
        exit 1
    fi
    ok "Docker found"

    if ! docker compose version &>/dev/null 2>&1 && ! docker-compose --version &>/dev/null 2>&1; then
        err "Docker Compose is not installed."
        exit 1
    fi
    ok "Docker Compose found"

    if [ ! -f "$ROOT_DIR/Server/.env" ]; then
        header "Configuration"
        cp "$ROOT_DIR/Server/.env.example" "$ROOT_DIR/Server/.env"
        SECRET=$(python3 -c "import secrets; print(secrets.token_hex(32))" 2>/dev/null || openssl rand -hex 32 2>/dev/null || echo "change-me-to-a-random-key")
        sed -i "s/generate-a-random-key-here/$SECRET/" "$ROOT_DIR/Server/.env"

        read -rp "    Admin username [admin]: " ADMIN_USER; ADMIN_USER=${ADMIN_USER:-admin}
        read -rp "    Admin email [admin@localhost]: " ADMIN_EMAIL; ADMIN_EMAIL=${ADMIN_EMAIL:-admin@localhost}
        read -rsp "    Admin password [admin123]: " ADMIN_PASS; echo ""; ADMIN_PASS=${ADMIN_PASS:-admin123}

        sed -i "s/ADMIN_USERNAME=admin/ADMIN_USERNAME=$ADMIN_USER/; s/ADMIN_EMAIL=admin@localhost/ADMIN_EMAIL=$ADMIN_EMAIL/; s/ADMIN_PASSWORD=admin123/ADMIN_PASSWORD=$ADMIN_PASS/" "$ROOT_DIR/Server/.env"

        if select_option "Database?" "SQLite (simple)" "MySQL (advanced)"; then
            sed -i "s|^DATABASE_URL=|# DATABASE_URL=|; s|^DB_HOST=|# DB_HOST=|" "$ROOT_DIR/Server/.env"
            echo "DATABASE_URL=sqlite:///instance/syncnotes.db" >> "$ROOT_DIR/Server/.env"
            ok "Using SQLite"
        else
            read -rp "    DB host [localhost]: " DB_HOST; DB_HOST=${DB_HOST:-localhost}
            read -rp "    DB port [3306]: " DB_PORT; DB_PORT=${DB_PORT:-3306}
            read -rp "    DB name [syncnotes]: " DB_NAME; DB_NAME=${DB_NAME:-syncnotes}
            read -rp "    DB user [syncnotes]: " DB_USER; DB_USER=${DB_USER:-syncnotes}
            read -rsp "    DB password: " DB_PASS; echo ""
            sed -i "s/DB_HOST=localhost/DB_HOST=$DB_HOST/; s/DB_PORT=3306/DB_PORT=$DB_PORT/; s/DB_NAME=syncnotes/DB_NAME=$DB_NAME/; s/DB_USER=syncnotes/DB_USER=$DB_USER/; s/DB_PASSWORD=your-db-password-here/DB_PASSWORD=$DB_PASS/" "$ROOT_DIR/Server/.env"
            ok "MySQL configured"
        fi
        ok ".env created"
    else
        info "Server/.env exists, keeping it"
    fi

    mkdir -p "$ROOT_DIR/Server/instance" "$ROOT_DIR/Server/uploads"

    header "Starting Server"
    cd "$ROOT_DIR/Server"
    if docker compose version &>/dev/null 2>&1; then
        docker compose up -d
    else
        docker-compose up -d
    fi

    ADMIN_USER=$(grep ADMIN_USERNAME "$ROOT_DIR/Server/.env" | cut -d= -f2)
    ADMIN_PASS=$(grep ADMIN_PASSWORD "$ROOT_DIR/Server/.env" | cut -d= -f2)

    echo ""
    ok "Server is running!"
    echo ""
    echo -e "    ${CYAN}URL:${NC}      http://localhost:2394"
    echo -e "    ${CYAN}Login:${NC}    $ADMIN_USER / $ADMIN_PASS"
    echo ""
    echo -e "    ${YELLOW}stop:${NC}    cd Server && docker compose down"
    echo -e "    ${YELLOW}logs:${NC}    cd Server && docker compose logs -f"
    echo ""
}

# ── App Installation ───────────────────────────────────────────────────────

install_app() {
    echo ""
    header "Desktop App Installation"

    if ! command -v cargo &>/dev/null; then
        info "Installing Rust via rustup..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
        ok "Rust installed"
    else
        ok "Rust $(cargo --version | cut -d' ' -f2)"
    fi

    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        setup_pkg_manager
        header "System Dependencies"
        MISSING=""
        for pkg in "${PKGS_APP[@]}"; do
            if ! dpkg -s "$pkg" &>/dev/null 2>&1 && ! pacman -Qi "$pkg" &>/dev/null 2>&1 && ! rpm -q "$pkg" &>/dev/null 2>&1; then
                MISSING="$MISSING $pkg"
            fi
        done 2>/dev/null || true

        if [ -n "$MISSING" ]; then
            warn "Missing:${MISSING}"
            if confirm "Install missing packages?"; then
                $PKG_UPDATE
                IFS=" " read -ra PKG_ARRAY <<< "$MISSING"
                install_packages "${PKG_ARRAY[@]}"
                ok "Dependencies installed"
            fi
        else
            ok "All dependencies present"
        fi
    fi

    header "Building"
    info "Compiling (this takes a few minutes)..."
    cd "$ROOT_DIR/App"
    cargo clean --quiet 2>/dev/null
    cargo build --release

    BINARY="$ROOT_DIR/App/target/release/syncnotes-app"
    if [ ! -f "$BINARY" ]; then
        err "Build failed"
        exit 1
    fi
    ok "Build complete"

    header "Installing"
    INSTALL_DIR="/usr/local/bin"
    if [ ! -w "$INSTALL_DIR" ]; then
        INSTALL_DIR="$HOME/.local/bin"
        mkdir -p "$INSTALL_DIR"
    fi
    cp "$BINARY" "$INSTALL_DIR/syncnotes"
    cp "$ROOT_DIR/logo.png" "$INSTALL_DIR/logo.png"
    ok "Binary → $INSTALL_DIR/syncnotes"
    ok "Logo   → $INSTALL_DIR/logo.png"

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
    ok "Desktop entry created"

    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        local target
        SHELL_CONFIG="$HOME/.$(basename "$SHELL")rc"
        if [ -f "$SHELL_CONFIG" ] || [ -f "$HOME/.profile" ]; then
            target="${SHELL_CONFIG:-$HOME/.profile}"
            if ! grep -q "export PATH=\"\$PATH:$INSTALL_DIR\"" "$target" 2>/dev/null; then
                echo "" >> "$target"
                echo "export PATH=\"\$PATH:$INSTALL_DIR\"" >> "$target"
                ok "Added to PATH in $target"
                info "Restart your shell or run: source $target"
            fi
        fi
    fi

    echo ""
    ok "SyncNotes installed!"
    echo ""
    echo -e "    ${CYAN}Run:${NC}  syncnotes"
    echo ""
    echo "    First run will guide you through setup."
    echo "    Default server: https://notes.huebler.tech"
    echo ""
}

# ── Welcome ────────────────────────────────────────────────────────────────

clear
echo ""
echo -e "  ${CYAN}┌──────────────────────────────────────────┐${NC}"
echo -e "  ${CYAN}│${NC}          ${BLUE}SyncNotes Installer${NC}            ${CYAN}│${NC}"
echo -e "  ${CYAN}└──────────────────────────────────────────┘${NC}"
echo ""

if select_option "What would you like to install?" \
    "Server (Docker)" "Desktop App (Rust)"; then
    install_server
else
    install_app
fi

#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'; GREEN='\033[0;32m'; BLUE='\033[0;34m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'; BOLD='\033[1m'; DIM='\033[2m'; NC='\033[0m'
REPO="akuras22/SyncNotes"

info()  { echo -e "  ${BLUE}→${NC} $*"; }
ok()    { echo -e "  ${GREEN}✔${NC} $*"; }
warn()  { echo -e "  ${YELLOW}⚠${NC} $*"; }
err()   { echo -e "  ${RED}✘${NC} $*"; }
step()  { echo -e "\n  ${CYAN}${BOLD}$*${NC}"; }
sub()   { echo -e "  ${DIM}$*${NC}"; }

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"

finish() {
    echo ""
    echo -e "  ${GREEN}✔${NC} ${BOLD}All done.${NC}"
    echo ""
}

banner() {
    clear 2>/dev/null || true
    echo -e "  ${CYAN}╭──────────────────────────────────────────╮${NC}"
    echo -e "  ${CYAN}│${NC}    ${BOLD}${BLUE}SyncNotes${NC} ${DIM}— installer${NC}                 ${CYAN}│${NC}"
    echo -e "  ${CYAN}╰──────────────────────────────────────────╯${NC}"
}

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
        if [[ -z $key ]]; then
            echo ""; return $selected
        elif [[ $key == $'\033' ]]; then
            read -rsn 2 -t 0.005 seq || true
            case "${seq:-}" in
                '[A') selected=0 ;;
                '[B') selected=1 ;;
            esac
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

spin_run() {
    # spin_run "message" cmd args...
    local msg="$1"; shift
    local frames='⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏'
    local logf; logf=$(mktemp)
    ("$@" >"$logf" 2>&1) &
    local pid=$!
    local i=0
    if [ -t 1 ]; then
        while kill -0 "$pid" 2>/dev/null; do
            i=$(((i + 1) % ${#frames}))
            printf "\r  ${CYAN}%s${NC} %s" "${frames:$i:1}" "$msg"
            sleep 0.08
        done
    fi
    if wait "$pid"; then
        printf "\r  ${GREEN}✔${NC} %s\n" "$msg"
        rm -f "$logf"
        return 0
    else
        printf "\r  ${RED}✘${NC} %s\n" "$msg"
        sed 's/^/      /' "$logf" >&2
        rm -f "$logf"
        return 1
    fi
}

# ── Distro detection ───────────────────────────────────────────────────────

detect_distro() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release; echo "$ID"
    else echo "unknown"; fi
}

PKG_MANAGER=""; PKG_UPDATE=""; PKG_INSTALL=""
PKGS_APP=(libgtk-3-dev libwebkit2gtk-4.1-dev librsvg2-dev libxdo-dev)

setup_pkg_manager() {
    local distro
    distro=$(detect_distro)
    case "$distro" in
        debian|ubuntu|linuxmint|pop|elementary|zorin|raspbian)
            PKG_MANAGER="apt"; PKG_UPDATE="sudo apt-get update -qq"
            PKG_INSTALL="sudo apt-get install -y -qq"
            PKGS_APP=(libgtk-3-dev libwebkit2gtk-4.1-dev librsvg2-dev libxdo-dev) ;;
        arch|manjaro|endeavouros|arco|archarm|cachyos)
            PKG_MANAGER="pacman"; PKG_UPDATE="sudo pacman -Sy --noconfirm"
            PKG_INSTALL="sudo pacman -S --noconfirm"
            PKGS_APP=(gtk3 webkit2gtk-4.1 librsvg xdotool) ;;
        fedora)
            PKG_MANAGER="dnf"; PKG_UPDATE="sudo dnf check-update -q || true"
            PKG_INSTALL="sudo dnf install -y"
            PKGS_APP=(gtk3-devel webkit2gtk4.1-devel librsvg2-devel libxdo-devel) ;;
        rhel|centos|rocky|almalinux)
            PKG_MANAGER="dnf"; PKG_UPDATE="sudo dnf check-update -q || true"
            PKG_INSTALL="sudo dnf install -y"
            PKGS_APP=(gtk3-devel webkit2gtk4.1-devel librsvg2-devel libxdo-devel) ;;
        opensuse*|suse)
            PKG_MANAGER="zypper"; PKG_UPDATE="sudo zypper refresh"
            PKG_INSTALL="sudo zypper install -y"
            PKGS_APP=(gtk3-devel webkit2gtk4-devel librsvg-devel libxdo-devel) ;;
        void)
            PKG_MANAGER="xbps"; PKG_UPDATE="sudo xbps-install -S"
            PKG_INSTALL="sudo xbps-install -y"
            PKGS_APP=(gtk3-devel webkit2gtk-devel librsvg-devel libxdo-devel) ;;
        alpine)
            PKG_MANAGER="apk"; PKG_UPDATE="sudo apk update"
            PKG_INSTALL="sudo apk add"
            PKGS_APP=(gtk3-dev webkit2gtk-dev librsvg-dev libxdo-dev) ;;
        solus)
            PKG_MANAGER="eopkg"; PKG_UPDATE="sudo eopkg update-repo"
            PKG_INSTALL="sudo eopkg install"
            PKGS_APP=(libgtk-3-devel libwebkit2gtk-4.1-devel librsvg-devel libxdo-devel) ;;
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
    step "Server Installation"

    if ! command -v docker &>/dev/null; then
        err "Docker is not installed."
        sub "Install: https://docs.docker.com/engine/install/"
        exit 1
    fi
    ok "Docker found"

    if ! docker compose version &>/dev/null 2>&1 && ! docker-compose --version &>/dev/null 2>&1; then
        err "Docker Compose is not installed."
        exit 1
    fi
    ok "Docker Compose found"

    if [ ! -f "$ROOT_DIR/Server/.env" ]; then
        step "Configuration"
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

    step "Starting Server"
    cd "$ROOT_DIR/Server"
    if docker compose version &>/dev/null 2>&1; then
        spin_run "Pulling images and starting containers" docker compose up -d
    else
        spin_run "Pulling images and starting containers" docker-compose up -d
    fi

    ADMIN_USER=$(grep ADMIN_USERNAME "$ROOT_DIR/Server/.env" | cut -d= -f2)
    ADMIN_PASS=$(grep ADMIN_PASSWORD "$ROOT_DIR/Server/.env" | cut -d= -f2)

    step "Server is running"
    echo -e "    ${CYAN}URL:${NC}      http://localhost:2394"
    echo -e "    ${CYAN}Login:${NC}    $ADMIN_USER / $ADMIN_PASS"
    echo ""
    echo -e "    ${DIM}stop:${NC}    cd Server && docker compose down"
    echo -e "    ${DIM}logs:${NC}    cd Server && docker compose logs -f"
    finish
}

# ── App Installation ───────────────────────────────────────────────────────

fetch_latest_release_json() {
    curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null
}

release_asset_url() {
    # release_asset_url <json> <substring>
    echo "$1" | grep -o '"browser_download_url": *"[^"]*"' \
        | sed -E 's/.*"(https:[^"]+)"/\1/' \
        | grep "$2" | head -1
}

release_tag() {
    echo "$1" | grep -o '"tag_name": *"[^"]*"' | head -1 | sed -E 's/.*"([^"]+)"$/\1/'
}

install_dir_for_binary() {
    if [ -w "/usr/local/bin" ]; then
        echo "/usr/local/bin"
    else
        mkdir -p "$HOME/.local/bin"
        echo "$HOME/.local/bin"
    fi
}

write_desktop_entry() {
    local install_dir="$1"
    local desktop_dir="$HOME/.local/share/applications"
    mkdir -p "$desktop_dir"
    cat > "$desktop_dir/syncnotes.desktop" <<EOF
[Desktop Entry]
Name=SyncNotes
Comment=Sync your Rnotes to SyncNotes server
Exec=$install_dir/syncnotes
Terminal=false
Type=Application
Categories=Utility;
Icon=$install_dir/syncnotes-icon.png
StartupWMClass=com.syncnotes.desktop
EOF
}

add_to_path() {
    local install_dir="$1"
    [[ ":$PATH:" == *":$install_dir:"* ]] && return 0
    local target="$HOME/.$(basename "$SHELL")rc"
    [ -f "$target" ] || target="$HOME/.profile"
    if ! grep -q "PATH=\"\$PATH:$install_dir\"" "$target" 2>/dev/null; then
        { echo ""; echo "export PATH=\"\$PATH:$install_dir\""; } >> "$target"
        ok "Added $install_dir to PATH in $(basename "$target")"
        sub "restart your shell, or run: source $target"
    fi
}

app_success_message() {
    local version="$1"
    echo ""
    ok "SyncNotes ${version:+$version }installed!"
    echo ""
    echo -e "    ${CYAN}Run:${NC}  syncnotes"
    echo ""
    echo "    First run will guide you through setup."
    echo "    Default server: https://notes.huebler.tech"
    finish
}

install_app_prebuilt() {
    step "Downloading Prebuilt Binary"

    local arch; arch=$(uname -m)
    if [ "$arch" != "x86_64" ]; then
        warn "Only x86_64 binaries are published (detected: $arch)."
        return 1
    fi

    info "Checking latest release..."
    local json; json=$(fetch_latest_release_json) || true
    if [ -z "${json:-}" ] || ! echo "$json" | grep -q browser_download_url; then
        warn "Could not reach GitHub releases."
        return 1
    fi

    local url; url=$(release_asset_url "$json" "linux")
    if [ -z "$url" ]; then
        warn "No Linux binary found in the latest release."
        return 1
    fi
    local version; version=$(release_tag "$json")
    ok "Latest release: ${version:-unknown}"

    local install_dir; install_dir=$(install_dir_for_binary)
    spin_run "Downloading syncnotes ($install_dir)" curl -fsSL "$url" -o "$install_dir/syncnotes" || return 1
    chmod +x "$install_dir/syncnotes"
    ok "Binary → $install_dir/syncnotes"

    cp "$ROOT_DIR/syncnotes-icon.png" "$install_dir/syncnotes-icon.png" 2>/dev/null || true
    write_desktop_entry "$install_dir"
    ok "Desktop entry created"
    add_to_path "$install_dir"

    app_success_message "$version"
    return 0
}

install_app_source() {
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
        step "System Dependencies"
        PC_NAMES=("gtk+-3.0" "webkit2gtk-4.1" "librsvg-2.0")
        MISSING_PKGS=()
        for pc in "${PC_NAMES[@]}"; do
            if ! pkg-config --exists "$pc" 2>/dev/null; then
                MISSING_PKGS+=("$pc")
            fi
        done

        if ! pkg-config --exists appindicator3 2>/dev/null && ! pkg-config --exists appindicator3-0.1 2>/dev/null && ! pkg-config --exists ayatana-appindicator3 2>/dev/null; then
            MISSING_PKGS+=("appindicator3")
        fi

        if [ ${#MISSING_PKGS[@]} -gt 0 ]; then
            warn "Missing (${MISSING_PKGS[*]})"
            if confirm "Install missing packages?"; then
                $PKG_UPDATE
                install_packages "${PKGS_APP[@]}"
                ok "Dependencies installed"
            fi
        else
            ok "All dependencies present"
        fi

        # Tray icon library
        if ! pkg-config --exists appindicator3 2>/dev/null && ! pkg-config --exists appindicator3-0.1 2>/dev/null && ! pkg-config --exists ayatana-appindicator3 2>/dev/null; then
            case "$PKG_MANAGER" in
                apt) pkg="libappindicator3-dev"; dpkg -s "$pkg" &>/dev/null || $PKG_INSTALL "$pkg" 2>/dev/null || { pkg="libayatana-appindicator3-dev"; dpkg -s "$pkg" &>/dev/null || $PKG_INSTALL "$pkg" 2>/dev/null; } || true ;;
                pacman) pkg="libappindicator-gtk3"; pacman -Qi "$pkg" &>/dev/null || $PKG_INSTALL "$pkg" || true ;;
                dnf) pkg="libappindicator-gtk3-devel"; rpm -q "$pkg" &>/dev/null || $PKG_INSTALL "$pkg" 2>/dev/null || true ;;
                zypper) pkg="libappindicator3-devel"; rpm -q "$pkg" &>/dev/null || $PKG_INSTALL "$pkg" 2>/dev/null || true ;;
            esac
        fi
    fi

    step "Building"
    cd "$ROOT_DIR/App"
    spin_run "Compiling (this can take a few minutes)" cargo build --release

    BINARY="$ROOT_DIR/App/target/release/syncnotes-app"
    if [ ! -f "$BINARY" ]; then
        err "Build failed"
        exit 1
    fi

    step "Installing"
    local install_dir; install_dir=$(install_dir_for_binary)
    cp "$BINARY" "$install_dir/syncnotes"
    cp "$ROOT_DIR/syncnotes-icon.png" "$install_dir/syncnotes-icon.png"
    ok "Binary → $install_dir/syncnotes"
    ok "Icon   → $install_dir/syncnotes-icon.png"

    write_desktop_entry "$install_dir"
    ok "Desktop entry created"
    add_to_path "$install_dir"

    app_success_message ""
}

install_app() {
    step "Desktop App Installation"

    for existing in "/usr/local/bin/syncnotes" "$HOME/.local/bin/syncnotes"; do
        if [ -f "$existing" ]; then
            warn "SyncNotes is already installed at $existing"
            if ! confirm "Reinstall / update it?"; then
                echo ""
                info "Nothing to do."
                exit 0
            fi
            break
        fi
    done

    local use_prebuilt=1
    if [[ "$OSTYPE" != "linux-gnu"* ]] || ! command -v curl &>/dev/null; then
        use_prebuilt=0
    elif select_option "How would you like to install?" \
        "Download prebuilt binary (fast, recommended)" "Build from source (for developers)"; then
        use_prebuilt=1
    else
        use_prebuilt=0
    fi

    if [ "$use_prebuilt" = "1" ]; then
        if install_app_prebuilt; then
            return 0
        fi
        warn "Falling back to building from source."
    fi

    install_app_source
}

# ── Welcome ────────────────────────────────────────────────────────────────

banner

if select_option "What would you like to install?" \
    "Desktop App (Rust)" "Server (Docker)"; then
    install_app
else
    install_server
fi

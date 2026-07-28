@echo off
setlocal enabledelayedexpansion

echo ========================================
echo        SyncNotes Installer (Windows)
echo ========================================
echo.

:menu
echo What would you like to install?
echo   1) Server (Docker)
echo   2) Desktop App (Rust)
echo.
set /p choice="Select [1/2]: "

if "%choice%"=="1" goto server
if "%choice%"=="2" goto app
echo Please enter 1 or 2.
goto menu

:: ── Server ──────────────────────────────────────────────────────────────

:server
echo.
echo [INFO] Starting server installation...

where docker >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERR] Docker is not installed.
    echo   Download it from: https://docs.docker.com/desktop/setup/install/windows-install/
    pause
    exit /b 1
)

if not exist "Server\.env" (
    echo [INFO] Creating Server\.env from .env.example...
    copy "Server\.env.example" "Server\.env" >nul

    echo.
    echo -- Server Configuration --
    set /p ADMIN_USER="Admin username [admin]: "
    if "!ADMIN_USER!"=="" set ADMIN_USER=admin
    set /p ADMIN_EMAIL="Admin email [admin@localhost]: "
    if "!ADMIN_EMAIL!"=="" set ADMIN_EMAIL=admin@localhost
    set /p ADMIN_PASS="Admin password [admin123]: "
    if "!ADMIN_PASS!"=="" set ADMIN_PASS=admin123

    powershell -Command "(Get-Content 'Server\.env') -replace 'ADMIN_USERNAME=admin', 'ADMIN_USERNAME=%ADMIN_USER%' | Set-Content 'Server\.env'"
    powershell -Command "(Get-Content 'Server\.env') -replace 'ADMIN_EMAIL=admin@localhost', 'ADMIN_EMAIL=%ADMIN_EMAIL%' | Set-Content 'Server\.env'"
    powershell -Command "(Get-Content 'Server\.env') -replace 'ADMIN_PASSWORD=admin123', 'ADMIN_PASSWORD=%ADMIN_PASS%' | Set-Content 'Server\.env'"

    echo.
    echo Use SQLite (simple) or MySQL?
    echo   1) SQLite
    echo   2) MySQL
    set /p db_choice="Select [1/2]: "
    if "!db_choice!"=="2" goto mysql_config

    echo DATABASE_URL=sqlite:///instance/syncnotes.db >> "Server\.env"
    echo [OK] Using SQLite.
    goto docker_start

    :mysql_config
    set /p DB_HOST="DB host [localhost]: "
    if "!DB_HOST!"=="" set DB_HOST=localhost
    set /p DB_PORT="DB port [3306]: "
    if "!DB_PORT!"=="" set DB_PORT=3306
    set /p DB_NAME="DB name [syncnotes]: "
    if "!DB_NAME!"=="" set DB_NAME=syncnotes
    set /p DB_USER="DB user [syncnotes]: "
    if "!DB_USER!"=="" set DB_USER=syncnotes
    set /p DB_PASS="DB password: "

    powershell -Command "(Get-Content 'Server\.env') -replace 'DB_HOST=localhost', 'DB_HOST=%DB_HOST%' | Set-Content 'Server\.env'"
    powershell -Command "(Get-Content 'Server\.env') -replace 'DB_PORT=3306', 'DB_PORT=%DB_PORT%' | Set-Content 'Server\.env'"
    powershell -Command "(Get-Content 'Server\.env') -replace 'DB_NAME=syncnotes', 'DB_NAME=%DB_NAME%' | Set-Content 'Server\.env'"
    powershell -Command "(Get-Content 'Server\.env') -replace 'DB_USER=syncnotes', 'DB_USER=%DB_USER%' | Set-Content 'Server\.env'"
    powershell -Command "(Get-Content 'Server\.env') -replace 'DB_PASSWORD=your-db-password-here', 'DB_PASSWORD=%DB_PASS%' | Set-Content 'Server\.env'"
    echo [OK] MySQL configured.

    :docker_start
    echo [OK] .env created and configured.
) else (
    echo [INFO] Server\.env already exists, keeping it.
)

if not exist "Server\instance" mkdir "Server\instance"
if not exist "Server\uploads" mkdir "Server\uploads"

echo.
echo [INFO] Starting server via Docker Compose...
cd Server
docker compose up -d

echo.
echo [OK] Server is running!
echo.
echo   Access it at:  http://localhost:2394
echo.
echo   To stop:       docker compose down
echo   To view logs:  docker compose logs -f
echo.
pause
exit /b 0

:: ── App ─────────────────────────────────────────────────────────────────

:app
echo.
echo [INFO] Starting desktop app installation...

where cargo >nul 2>&1
if %errorlevel% neq 0 (
    echo [INFO] Rust is not installed. Installing via rustup...
    curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs -o rustup-init.exe
    rustup-init.exe -y
    del rustup-init.exe
    call "%USERPROFILE%\.cargo\env"
    echo [OK] Rust installed.
) else (
    echo [OK] Rust is already installed.
)

echo.
echo [INFO] Building SyncNotes app (this may take a few minutes)...
cd App
cargo build --release

set BINARY=App\target\release\syncnotes-app.exe
if not exist "!BINARY!" (
    if exist "target\release\syncnotes-app.exe" set "BINARY=target\release\syncnotes-app.exe"
)

if not exist "!BINARY!" (
    echo [ERR] Build failed - binary not found.
    pause
    exit /b 1
)

echo.
echo [INFO] Installing binary...

set INSTALL_DIR=%LOCALAPPDATA%\Programs\SyncNotes
if not exist "!INSTALL_DIR!" mkdir "!INSTALL_DIR!"

copy "!BINARY!" "!INSTALL_DIR!\syncnotes.exe" >nul
echo [OK] Installed to !INSTALL_DIR!\syncnotes.exe

echo [INFO] Creating Start Menu shortcut...
set SCRIPT="%TEMP%\SyncNotesShortcut.ps1"
echo $ws = New-Object -ComObject WScript.Shell > %SCRIPT%
echo $s = $ws.CreateShortcut("$env:APPDATA\Microsoft\Windows\Start Menu\Programs\SyncNotes.lnk") >> %SCRIPT%
echo $s.TargetPath = "!INSTALL_DIR!\syncnotes.exe" >> %SCRIPT%
echo $s.Save() >> %SCRIPT%
powershell -ExecutionPolicy Bypass -File %SCRIPT% >nul 2>&1
del %SCRIPT%
echo [OK] Shortcut created.

set "PATH_USER=%PATH%"
echo !PATH_USER! | findstr /C:"!INSTALL_DIR!" >nul
if %errorlevel% neq 0 (
    echo.
    echo Add to PATH manually or run:
    echo   setx PATH "%%PATH%%;!INSTALL_DIR!"
)

echo.
echo [OK] SyncNotes app installed successfully!
echo.
echo   Run it:        !INSTALL_DIR!\syncnotes.exe
echo   Settings:      syncnotes.exe --settings
echo.
echo   The app will guide you through setup on first run.
echo   It will connect to https://notes.huebler.tech by default.
echo.
pause
exit /b 0

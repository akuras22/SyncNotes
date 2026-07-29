@echo off
setlocal enabledelayedexpansion

for /F "delims=" %%a in ('echo prompt $E^|cmd') do set "ESC=%%a"
set "C_CYAN=%ESC%[36m"
set "C_BLUE=%ESC%[34m"
set "C_GREEN=%ESC%[32m"
set "C_YELLOW=%ESC%[33m"
set "C_RED=%ESC%[31m"
set "C_DIM=%ESC%[2m"
set "C_BOLD=%ESC%[1m"
set "C_RESET=%ESC%[0m"

set "REPO=akuras22/SyncNotes"

echo.
echo   %C_CYAN%%C_BOLD%SyncNotes%C_RESET% %C_DIM%- installer%C_RESET%
echo   %C_CYAN%========================================%C_RESET%
echo.

:menu
echo   What would you like to install?
echo     1^) Desktop App
echo     2^) Server (Docker)
echo.
set /p choice="  Select [1/2]: "

if "%choice%"=="1" goto app
if "%choice%"=="2" goto server
echo   Please enter 1 or 2.
goto menu

:: ── Server ──────────────────────────────────────────────────────────────

:server
echo.
echo   %C_CYAN%%C_BOLD%Server Installation%C_RESET%

where docker >nul 2>&1
if %errorlevel% neq 0 (
    echo   %C_RED%x%C_RESET% Docker is not installed.
    echo     Download it from: https://docs.docker.com/desktop/setup/install/windows-install/
    pause
    exit /b 1
)
echo   %C_GREEN%v%C_RESET% Docker found

if not exist "Server\.env" (
    echo.
    echo   %C_CYAN%Configuration%C_RESET%
    copy "Server\.env.example" "Server\.env" >nul

    set /p ADMIN_USER="    Admin username [admin]: "
    if "!ADMIN_USER!"=="" set ADMIN_USER=admin
    set /p ADMIN_EMAIL="    Admin email [admin@localhost]: "
    if "!ADMIN_EMAIL!"=="" set ADMIN_EMAIL=admin@localhost
    set /p ADMIN_PASS="    Admin password [admin123]: "
    if "!ADMIN_PASS!"=="" set ADMIN_PASS=admin123

    powershell -NoProfile -Command "(Get-Content 'Server\.env') -replace 'ADMIN_USERNAME=admin', 'ADMIN_USERNAME=%ADMIN_USER%' | Set-Content 'Server\.env'"
    powershell -NoProfile -Command "(Get-Content 'Server\.env') -replace 'ADMIN_EMAIL=admin@localhost', 'ADMIN_EMAIL=%ADMIN_EMAIL%' | Set-Content 'Server\.env'"
    powershell -NoProfile -Command "(Get-Content 'Server\.env') -replace 'ADMIN_PASSWORD=admin123', 'ADMIN_PASSWORD=%ADMIN_PASS%' | Set-Content 'Server\.env'"

    echo.
    echo     Database?
    echo       1^) SQLite (simple)
    echo       2^) MySQL (advanced)
    set /p db_choice="    Select [1/2]: "
    if "!db_choice!"=="2" goto mysql_config

    echo DATABASE_URL=sqlite:///instance/syncnotes.db >> "Server\.env"
    echo   %C_GREEN%v%C_RESET% Using SQLite
    goto docker_start

    :mysql_config
    set /p DB_HOST="    DB host [localhost]: "
    if "!DB_HOST!"=="" set DB_HOST=localhost
    set /p DB_PORT="    DB port [3306]: "
    if "!DB_PORT!"=="" set DB_PORT=3306
    set /p DB_NAME="    DB name [syncnotes]: "
    if "!DB_NAME!"=="" set DB_NAME=syncnotes
    set /p DB_USER="    DB user [syncnotes]: "
    if "!DB_USER!"=="" set DB_USER=syncnotes
    set /p DB_PASS="    DB password: "

    powershell -NoProfile -Command "(Get-Content 'Server\.env') -replace 'DB_HOST=localhost', 'DB_HOST=%DB_HOST%' | Set-Content 'Server\.env'"
    powershell -NoProfile -Command "(Get-Content 'Server\.env') -replace 'DB_PORT=3306', 'DB_PORT=%DB_PORT%' | Set-Content 'Server\.env'"
    powershell -NoProfile -Command "(Get-Content 'Server\.env') -replace 'DB_NAME=syncnotes', 'DB_NAME=%DB_NAME%' | Set-Content 'Server\.env'"
    powershell -NoProfile -Command "(Get-Content 'Server\.env') -replace 'DB_USER=syncnotes', 'DB_USER=%DB_USER%' | Set-Content 'Server\.env'"
    powershell -NoProfile -Command "(Get-Content 'Server\.env') -replace 'DB_PASSWORD=your-db-password-here', 'DB_PASSWORD=%DB_PASS%' | Set-Content 'Server\.env'"
    echo   %C_GREEN%v%C_RESET% MySQL configured

    :docker_start
    echo   %C_GREEN%v%C_RESET% .env created and configured
) else (
    echo   %C_DIM%.%C_RESET% Server\.env already exists, keeping it
)

if not exist "Server\instance" mkdir "Server\instance"
if not exist "Server\uploads" mkdir "Server\uploads"

echo.
echo   %C_BLUE%-^>%C_RESET% Building and starting server via Docker Compose...
cd Server
docker compose up -d --build
if %errorlevel% neq 0 (
    cd ..
    echo   %C_RED%x%C_RESET% Failed to start the server. Check the Docker output above.
    pause
    exit /b 1
)
cd ..

echo.
echo   %C_GREEN%%C_BOLD%Server is running%C_RESET%
echo.
echo     URL:      http://localhost:2394
echo     Login:    %ADMIN_USER%
echo.
echo     %C_DIM%stop:    cd Server ^&^& docker compose down%C_RESET%
echo     %C_DIM%logs:    cd Server ^&^& docker compose logs -f%C_RESET%
echo.
pause
exit /b 0

:: ── App ─────────────────────────────────────────────────────────────────

:app
echo.
echo   %C_CYAN%%C_BOLD%Desktop App Installation%C_RESET%

set "INSTALL_DIR=%LOCALAPPDATA%\Programs\SyncNotes"

if exist "%INSTALL_DIR%\syncnotes.exe" (
    echo   %C_YELLOW%*%C_RESET% SyncNotes is already installed at %INSTALL_DIR%
    set /p REINSTALL="    Reinstall / update it? [y/N]: "
    if /i not "!REINSTALL!"=="y" (
        echo   Nothing to do.
        pause
        exit /b 0
    )
)

echo.
echo   How would you like to install?
echo     1^) Download prebuilt binary (fast, recommended^)
echo     2^) Build from source (for developers^)
set /p method_choice="  Select [1/2]: "

if "%method_choice%"=="2" goto app_source

:app_prebuilt
echo.
echo   %C_BLUE%-^>%C_RESET% Checking latest release...

set "RELEASE_INFO=%TEMP%\syncnotes_release.txt"
powershell -NoProfile -Command ^
    "$ErrorActionPreference = 'Stop';" ^
    "try {" ^
    "  $r = Invoke-RestMethod -Uri 'https://api.github.com/repos/%REPO%/releases/latest' -Headers @{ 'User-Agent' = 'SyncNotes-Installer' };" ^
    "  $asset = $r.assets ^| Where-Object { $_.name -like '*windows*' } ^| Select-Object -First 1;" ^
    "  if (-not $asset) { 'NOASSET' ^| Out-File -Encoding ascii '%RELEASE_INFO%'; exit 1 }" ^
    "  $r.tag_name ^| Out-File -Encoding ascii '%RELEASE_INFO%';" ^
    "  $asset.browser_download_url ^| Out-File -Append -Encoding ascii '%RELEASE_INFO%';" ^
    "} catch { 'ERROR' ^| Out-File -Encoding ascii '%RELEASE_INFO%'; exit 1 }"

set "REL_TAG="
set "REL_URL="
if exist "%RELEASE_INFO%" (
    for /f "usebackq delims=" %%L in ("%RELEASE_INFO%") do (
        if not defined REL_TAG (set "REL_TAG=%%L") else if not defined REL_URL (set "REL_URL=%%L")
    )
)
del "%RELEASE_INFO%" >nul 2>&1

if not defined REL_URL (
    echo   %C_YELLOW%*%C_RESET% Could not find a Windows binary in the latest release.
    echo     Falling back to building from source.
    goto app_source
)

echo   %C_GREEN%v%C_RESET% Latest release: %REL_TAG%
if not exist "%INSTALL_DIR%" mkdir "%INSTALL_DIR%"

echo   %C_BLUE%-^>%C_RESET% Downloading syncnotes.exe...
powershell -NoProfile -Command "$ErrorActionPreference='Stop'; Invoke-WebRequest -Uri '%REL_URL%' -OutFile '%INSTALL_DIR%\syncnotes.exe'"
if %errorlevel% neq 0 (
    echo   %C_YELLOW%*%C_RESET% Download failed. Falling back to building from source.
    goto app_source
)
echo   %C_GREEN%v%C_RESET% Installed to %INSTALL_DIR%\syncnotes.exe
goto app_finish

:app_source
echo.
where cargo >nul 2>&1
if %errorlevel% neq 0 (
    echo   %C_BLUE%-^>%C_RESET% Rust is not installed. Installing via rustup...
    curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs -o rustup-init.exe
    rustup-init.exe -y
    del rustup-init.exe
    call "%USERPROFILE%\.cargo\env"
    echo   %C_GREEN%v%C_RESET% Rust installed
) else (
    echo   %C_GREEN%v%C_RESET% Rust is already installed
)

echo.
echo   %C_BLUE%-^>%C_RESET% Building SyncNotes app (this may take a few minutes)...
cd App
cargo build --release
set "BUILD_RESULT=%errorlevel%"
cd ..

if not "%BUILD_RESULT%"=="0" (
    echo   %C_RED%x%C_RESET% Build failed.
    pause
    exit /b 1
)

set "BINARY=App\target\release\syncnotes-app.exe"
if not exist "%BINARY%" (
    echo   %C_RED%x%C_RESET% Build finished but the binary was not found at %BINARY%.
    pause
    exit /b 1
)

echo.
echo   %C_BLUE%-^>%C_RESET% Installing binary...
if not exist "%INSTALL_DIR%" mkdir "%INSTALL_DIR%"
copy "%BINARY%" "%INSTALL_DIR%\syncnotes.exe" >nul
echo   %C_GREEN%v%C_RESET% Installed to %INSTALL_DIR%\syncnotes.exe

:app_finish
echo   %C_BLUE%-^>%C_RESET% Creating Start Menu shortcut...
set "SHORTCUT_SCRIPT=%TEMP%\SyncNotesShortcut.ps1"
echo $ws = New-Object -ComObject WScript.Shell > "%SHORTCUT_SCRIPT%"
echo $s = $ws.CreateShortcut("$env:APPDATA\Microsoft\Windows\Start Menu\Programs\SyncNotes.lnk") >> "%SHORTCUT_SCRIPT%"
echo $s.TargetPath = "%INSTALL_DIR%\syncnotes.exe" >> "%SHORTCUT_SCRIPT%"
echo $s.Save() >> "%SHORTCUT_SCRIPT%"
powershell -NoProfile -ExecutionPolicy Bypass -File "%SHORTCUT_SCRIPT%" >nul 2>&1
del "%SHORTCUT_SCRIPT%" >nul 2>&1
echo   %C_GREEN%v%C_RESET% Shortcut created

powershell -NoProfile -Command ^
    "$dir = '%INSTALL_DIR%';" ^
    "$p = [Environment]::GetEnvironmentVariable('Path','User');" ^
    "if ($p -notlike ('*' + $dir + '*')) {" ^
    "  [Environment]::SetEnvironmentVariable('Path', $p.TrimEnd(';') + ';' + $dir, 'User');" ^
    "  exit 0" ^
    "} else { exit 1 }"
if %errorlevel% equ 0 (
    echo   %C_GREEN%v%C_RESET% Added to PATH ^(open a new terminal for this to take effect^)
) else (
    echo   %C_DIM%.%C_RESET% Already on PATH
)

echo.
echo   %C_GREEN%%C_BOLD%SyncNotes installed%C_RESET%
echo.
echo     Run it:   %INSTALL_DIR%\syncnotes.exe
echo     Or just:  syncnotes.exe   %C_DIM%(after opening a new terminal)%C_RESET%
echo.
echo     First run will guide you through setup.
echo     Default server: https://notes.huebler.tech
echo.
pause
exit /b 0

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

echo.
echo   %C_CYAN%%C_BOLD%SyncNotes%C_RESET% %C_DIM%- uninstaller%C_RESET%
echo   %C_CYAN%========================================%C_RESET%
echo.

:menu
echo   What would you like to uninstall?
echo     1^) Desktop App
echo     2^) Server (Docker)
echo.
set /p choice="  Select [1/2]: "

if "%choice%"=="1" goto app
if "%choice%"=="2" goto server
echo   Please enter 1 or 2.
goto menu

:: ── Uninstall Server ─────────────────────────────────────────────────────

:server
echo.
echo   %C_CYAN%%C_BOLD%Server Uninstall%C_RESET%

set "FOUND_ANYTHING=0"

if exist "Server" (
    pushd "Server"
) else (
    pushd "%~dp0Server" 2>nul || pushd "%~dp0"
)

where docker >nul 2>&1
if %errorlevel% equ 0 (
    docker compose down -v >nul 2>&1
    docker rm -f syncnotes >nul 2>&1
    echo   %C_GREEN%v%C_RESET% Docker container stopped and removed
    set "FOUND_ANYTHING=1"

    for /f "delims=" %%v in ('docker volume ls --format "{{.Name}}" 2^>nul ^| findstr /i syncnotes') do (
        set "HAS_VOLUMES=1"
    )
    if defined HAS_VOLUMES (
        set /p delvol="    Remove leftover Docker volumes? [y/N]: "
        if /i "!delvol!"=="y" (
            for /f "delims=" %%v in ('docker volume ls --format "{{.Name}}" 2^>nul ^| findstr /i syncnotes') do (
                docker volume rm "%%v" >nul 2>&1
            )
            echo   %C_GREEN%v%C_RESET% Volumes removed
        )
    )
) else (
    echo   %C_DIM%.%C_RESET% Docker is not installed - nothing to stop
)

if exist ".env" (
    set /p delenv="    Remove .env configuration file? [y/N]: "
    if /i "!delenv!"=="y" (
        del ".env" >nul
        echo   %C_GREEN%v%C_RESET% .env removed
        set "FOUND_ANYTHING=1"
    )
)

if exist "uploads" (
    set /p deluploads="    Remove uploaded files (uploads folder)? [y/N]: "
    if /i "!deluploads!"=="y" (
        rmdir /s /q "uploads" >nul
        echo   %C_GREEN%v%C_RESET% Uploads removed
        set "FOUND_ANYTHING=1"
    )
)

if exist "instance" (
    set /p deldb="    Remove database files (instance folder)? [y/N]: "
    if /i "!deldb!"=="y" (
        rmdir /s /q "instance" >nul
        echo   %C_GREEN%v%C_RESET% Database files removed
        set "FOUND_ANYTHING=1"
    )
)

popd

if "%FOUND_ANYTHING%"=="0" (
    echo   %C_DIM%.%C_RESET% Nothing found to remove.
)

echo.
echo   %C_GREEN%%C_BOLD%Server uninstalled.%C_RESET%
echo.
pause
exit /b 0

:: ── Uninstall App ────────────────────────────────────────────────────────

:app
echo.
echo   %C_CYAN%%C_BOLD%Desktop App Uninstall%C_RESET%

set "FOUND_ANYTHING=0"

taskkill /f /im syncnotes.exe >nul 2>&1
if %errorlevel% equ 0 (
    echo   %C_GREEN%v%C_RESET% Stopped running instance
)

set "INSTALL_DIR=%LOCALAPPDATA%\Programs\SyncNotes"
if exist "%INSTALL_DIR%" (
    set /p delbin="    Remove app at %INSTALL_DIR%? [y/N]: "
    if /i "!delbin!"=="y" (
        rmdir /s /q "%INSTALL_DIR%"
        del "%APPDATA%\Microsoft\Windows\Start Menu\Programs\SyncNotes.lnk" >nul 2>&1
        echo   %C_GREEN%v%C_RESET% App and shortcut removed
        set "FOUND_ANYTHING=1"
    )
) else (
    echo   %C_DIM%.%C_RESET% No installed app found at %INSTALL_DIR%
)

set "CONFIG_DIR=%APPDATA%\syncnotes"
if exist "%CONFIG_DIR%" (
    set /p delcfg="    Remove configuration directory (%CONFIG_DIR%)? [y/N]: "
    if /i "!delcfg!"=="y" (
        rmdir /s /q "%CONFIG_DIR%"
        echo   %C_GREEN%v%C_RESET% Configuration removed
        set "FOUND_ANYTHING=1"
    )
) else (
    echo   %C_DIM%.%C_RESET% No configuration directory found
)

powershell -NoProfile -Command ^
    "$dir = '%INSTALL_DIR%';" ^
    "$p = [Environment]::GetEnvironmentVariable('Path','User');" ^
    "if ($p -like ('*' + $dir + '*')) {" ^
    "  $clean = ($p -split ';' ^| Where-Object { $_ -and $_ -ne $dir }) -join ';';" ^
    "  [Environment]::SetEnvironmentVariable('Path', $clean, 'User');" ^
    "  exit 0" ^
    "} else { exit 1 }"
if %errorlevel% equ 0 (
    echo   %C_GREEN%v%C_RESET% Removed from PATH
    set "FOUND_ANYTHING=1"
)

if "%FOUND_ANYTHING%"=="0" (
    echo   %C_DIM%.%C_RESET% Nothing found to remove.
)

echo.
echo   %C_GREEN%%C_BOLD%Desktop app uninstalled.%C_RESET%
echo.
echo     Note: the Rust toolchain (if installed via rustup) was left in place.
echo     To remove it: rustup self uninstall
echo.
pause
exit /b 0

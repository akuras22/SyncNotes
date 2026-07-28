@echo off
setlocal enabledelayedexpansion

echo ============================================
echo   SyncNotes Uninstaller (Windows)
echo ============================================
echo.

:menu
echo What would you like to uninstall?
echo   1) Server (Docker)
echo   2) Desktop App
echo.
set /p choice="Select [1/2]: "

if "%choice%"=="1" goto server
if "%choice%"=="2" goto app
echo Please enter 1 or 2.
goto menu

:: ── Uninstall Server ─────────────────────────────────────────────────────

:server
echo.
echo [INFO] Uninstalling SyncNotes server...

cd Server 2>nul

where docker >nul 2>&1
if %errorlevel% equ 0 (
    echo [INFO] Stopping and removing Docker container...
    docker compose down -v 2>nul
    docker rm -f syncnotes 2>nul
    echo [OK] Docker container removed.

    set /p delvol="Remove Docker volumes? (y/N): "
    if /i "!delvol!"=="y" (
        docker volume rm syncnotes_syncnotes_uploads 2>nul
        docker volume rm syncnotes_mysql_data 2>nul
        echo [OK] Volumes removed.
    )
) else (
    echo [INFO] Docker is not installed — nothing to do.
)

if exist ".env" (
    set /p delenv="Remove .env configuration file? (y/N): "
    if /i "!delenv!"=="y" (
        del ".env" >nul
        echo [OK] .env removed.
    )
)

if exist "uploads" (
    set /p deluploads="Remove uploaded files (uploads folder)? (y/N): "
    if /i "!deluploads!"=="y" (
        rmdir /s /q "uploads" >nul
        echo [OK] Uploads removed.
    )
)

if exist "instance" (
    set /p deldb="Remove database files (instance folder)? (y/N): "
    if /i "!deldb!"=="y" (
        rmdir /s /q "instance" >nul
        echo [OK] Database files removed.
    )
)

cd ..

echo.
echo [OK] Server uninstalled.
pause
exit /b 0

:: ── Uninstall App ────────────────────────────────────────────────────────

:app
echo.
echo [INFO] Uninstalling SyncNotes desktop app...

set INSTALL_DIR=%LOCALAPPDATA%\Programs\SyncNotes
if exist "!INSTALL_DIR!" (
    set /p delbin="Remove binary at !INSTALL_DIR!? (y/N): "
    if /i "!delbin!"=="y" (
        rmdir /s /q "!INSTALL_DIR!" >nul
        echo [OK] Binary removed.
    )
)

set CONFIG_DIR=%APPDATA%\syncnotes
if exist "!CONFIG_DIR!" (
    set /p delcfg="Remove configuration directory? (y/N): "
    if /i "!delcfg!"=="y" (
        rmdir /s /q "!CONFIG_DIR!" >nul
        echo [OK] Configuration removed.
    )
)

echo.
echo [OK] Desktop app uninstalled.
echo.
echo   Note: Rust toolchain was left installed.
echo   To remove it: rustup self uninstall
echo.
pause
exit /b 0

@echo off
rem Chest and Lever: start the game. Extra arguments are passed on (for example: play.bat --seed AMBER-DOOR-42).
rem Uses the newest build there is (release first, then debug); builds one if there is none yet.
cd /d "%~dp0"
set "BEVY_ASSET_ROOT=%~dp0"
if exist "target\release\lac.exe" (
    "target\release\lac.exe" %*
) else if exist "target\debug\lac.exe" (
    "target\debug\lac.exe" %*
) else (
    echo No build yet: building the game, this takes a while the first time...
    cargo run --release -- %*
)
if errorlevel 1 pause

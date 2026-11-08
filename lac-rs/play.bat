@echo off
rem Chest and Lever: start the game. Extra arguments are passed on (for example: play.bat --seed AMBER-DOOR-42).
rem Uses the newest build there is (release first, then debug); builds one if there is none yet.
rem The game runs from a copy in target\play, so a new build can be made while you play.
cd /d "%~dp0"
set "BEVY_ASSET_ROOT=%~dp0"
set "EXE="
if exist "target\debug\lac.exe" set "EXE=target\debug\lac.exe"
if exist "target\release\lac.exe" set "EXE=target\release\lac.exe"
if not defined EXE (
    echo No build yet: building the game, this takes a while the first time...
    cargo build --release || goto :failed
    set "EXE=target\release\lac.exe"
)
if not exist "target\play" mkdir "target\play"
copy /y "%EXE%" "target\play\lac.exe" >nul
"target\play\lac.exe" %*
if errorlevel 1 goto :failed
exit /b 0
:failed
pause

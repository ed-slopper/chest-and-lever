#!/usr/bin/env python3
"""No borrowed media in the repo (AGENTS.md section 13).

Board row F0. Every image, model, sound or video file must sit under assets/ and be named in
assets/CREDITS.md (who made it, the licence). Anywhere else such files are refused, because a screenshot or a
frame of Levers and Chests must never be committed. Run: `python tools/check_assets.py`. Python 3.8+, no packages.
"""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CREDITS = ROOT / "assets" / "CREDITS.md"
MEDIA = {
    # images
    ".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp", ".tga", ".dds", ".ktx2", ".psd", ".aseprite", ".svg",
    # models and voxels
    ".vox", ".glb", ".gltf", ".obj", ".fbx", ".blend", ".qb", ".ply",
    # sound and video
    ".wav", ".ogg", ".mp3", ".flac", ".opus", ".mp4", ".webm", ".mov", ".mkv",
}


def tracked_files():
    """Files git would commit (tracked plus untracked, ignored left out); falls back to a walk."""
    try:
        out = subprocess.run(
            ["git", "ls-files", "--cached", "--others", "--exclude-standard"],
            cwd=ROOT, capture_output=True, text=True, check=True,
        ).stdout
        return [Path(p) for p in out.splitlines() if p]
    except (OSError, subprocess.CalledProcessError):
        return [p.relative_to(ROOT) for p in ROOT.rglob("*") if p.is_file() and ".git" not in p.parts]


def main():
    errors = []
    credits = CREDITS.read_text(encoding="utf-8") if CREDITS.exists() else ""
    media = [p for p in tracked_files() if p.suffix.lower() in MEDIA]
    for p in media:
        posix = p.as_posix()
        if not posix.startswith("assets/"):
            errors.append(f"{posix}: media outside assets/ (pictures of the game stay off the repo; ours go in assets/)")
        elif posix[len("assets/"):] not in credits and posix not in credits:
            errors.append(f"{posix}: not named in assets/CREDITS.md")
    if errors:
        print(f"check_assets: {len(errors)} problem(s)")
        for e in errors:
            print(f"  - {e}")
        return 1
    print(f"check_assets: OK ({len(media)} media file(s), all credited)")
    return 0


if __name__ == "__main__":
    sys.exit(main())

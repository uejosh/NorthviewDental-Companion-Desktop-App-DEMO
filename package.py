#!/usr/bin/env python3
"""Build the Northview Office App and package it.

On Windows this produces dist/NorthviewOffice-<version>-setup.exe, a per-user installer
built with Inno Setup 6 (packaging/windows/northview-office.iss) that:
  * installs the app to %LOCALAPPDATA%\\Programs\\NorthviewOffice (no administrator needed)
  * creates Start-menu and (optional) Desktop shortcuts
  * registers the notification ID SturlDigital.NorthviewOffice under HKCU so desktop
    notifications show the Northview name and icon
  * adds an uninstaller under Settings > Apps
Inno Setup must be installed (https://jrsoftware.org/isdl.php, or `choco install innosetup`).

On Linux/macOS it produces a tar.gz of the native build.

Usage:
  python3 package.py                 # build for this machine
  python3 package.py --target x86_64-pc-windows-gnu
  python3 package.py --skip-build    # package an existing release build
"""

import argparse
import os
import pathlib
import platform
import re
import shutil
import subprocess
import sys
import tarfile

ROOT = pathlib.Path(__file__).resolve().parent
APP_ID = "SturlDigital.NorthviewOffice"

README = """Northview Office {version}

INSTALL
  ./install.sh   (adds Northview Office to your applications menu with its icon)

FIRST RUN
  On the sign-in screen click the connection link under the PIN box, choose
  "Website (live)" and enter the site address and device key shown on the website
  under Tools > Northview Office App. Staff PINs are set on that same page.

  "Demo data" mode runs on fictional sample patients for training.

Patient data from the website is kept in memory only and is never written to this PC.
"""


def version() -> str:
    m = re.search(r'^version\s*=\s*"([^"]+)"', (ROOT / "Cargo.toml").read_text(), re.M)
    return m.group(1) if m else "0.0.0"


def find_iscc() -> pathlib.Path | None:
    on_path = shutil.which("iscc") or shutil.which("ISCC.exe")
    if on_path:
        return pathlib.Path(on_path)
    for base in (os.environ.get("ProgramFiles(x86)"), os.environ.get("ProgramFiles"), os.environ.get("LOCALAPPDATA") and os.path.join(os.environ["LOCALAPPDATA"], "Programs")):
        if base:
            c = pathlib.Path(base) / "Inno Setup 6" / "ISCC.exe"
            if c.exists():
                return c
    return None


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--target", default=None)
    ap.add_argument("--skip-build", action="store_true")
    args = ap.parse_args()

    if not args.skip_build:
        cmd = ["cargo", "build", "--release"] + (["--target", args.target] if args.target else [])
        print("+", " ".join(cmd))
        subprocess.run(cmd, cwd=ROOT, check=True)

    windows = (args.target or "").endswith("windows-gnu") or (args.target or "").endswith("windows-msvc") or platform.system() == "Windows"
    rel = ROOT / "target" / (args.target or "") / "release"
    exe = rel / ("nvdental-office-app.exe" if windows else "nvdental-office-app")
    if not exe.exists():
        print(f"Build output not found: {exe}", file=sys.stderr)
        return 1

    ver = version()
    dist = ROOT / "dist"
    dist.mkdir(exist_ok=True)
    if windows:
        iscc = find_iscc()
        if not iscc:
            print("Inno Setup 6 (ISCC.exe) not found. Install it from https://jrsoftware.org/isdl.php", file=sys.stderr)
            return 1
        iss = ROOT / "packaging" / "windows" / "northview-office.iss"
        cmd = [str(iscc), "/Q", f"/DAppVersion={ver}", f"/DSourceExe={exe}", f"/DOutputDir={dist}", str(iss)]
        print("+", " ".join(cmd))
        subprocess.run(cmd, cwd=iss.parent, check=True)
        out = dist / f"NorthviewOffice-{ver}-setup.exe"
        if not out.exists():
            print(f"Installer not produced: {out}", file=sys.stderr)
            return 1
        print(f"Packaged {out.relative_to(ROOT)} ({out.stat().st_size // 1024} KB)")
        return 0

    # Linux / macOS: tar.gz with the binary, icons and install scripts
    stage = dist / f"NorthviewOffice-{ver}"
    if stage.exists():
        shutil.rmtree(stage)
    stage.mkdir()
    shutil.copy2(ROOT / "assets" / "icon.png", stage / "icon.png")
    (stage / "README.txt").write_text(README.format(version=ver))

    shutil.copy2(exe, stage / "northview-office")
    shutil.copy2(ROOT / "assets" / "icon-512.png", stage / "icon-512.png")
    for name in ("install.sh", "uninstall.sh"):
        shutil.copy2(ROOT / "packaging" / "linux" / name, stage / name)
    out = dist / f"NorthviewOffice-{ver}-{platform.system().lower()}-{platform.machine()}.tar.gz"
    with tarfile.open(out, "w:gz") as t:
        t.add(stage, arcname="NorthviewOffice")
    shutil.rmtree(stage)
    print(f"Packaged {out.relative_to(ROOT)} ({out.stat().st_size // 1024} KB)")
    return 0


if __name__ == "__main__":
    sys.exit(main())

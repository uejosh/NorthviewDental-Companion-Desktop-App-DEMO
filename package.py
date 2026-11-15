#!/usr/bin/env python3
"""Build the Northview Office App and package it.

On Windows (or with --target x86_64-pc-windows-gnu and a MinGW toolchain) this produces
dist/NorthviewOffice-<version>-windows.zip containing the executable and a per-user
PowerShell installer that:
  * copies the app to %LOCALAPPDATA%\\Programs\\NorthviewOffice
  * creates Desktop and Start-menu shortcuts
  * registers the notification ID SturlDigital.NorthviewOffice under HKCU so desktop
    notifications show the Northview name and icon
No administrator account is needed.

On Linux/macOS it produces a tar.gz of the native build.

Usage:
  python3 package.py                 # build for this machine
  python3 package.py --target x86_64-pc-windows-gnu
  python3 package.py --skip-build    # package an existing release build
"""

import argparse
import pathlib
import platform
import re
import shutil
import subprocess
import sys
import tarfile
import zipfile

ROOT = pathlib.Path(__file__).resolve().parent
APP_ID = "SturlDigital.NorthviewOffice"

INSTALL_PS1 = r"""
# Northview Office per-user installer. Run: right-click > Run with PowerShell
$ErrorActionPreference = 'Stop'
$appId   = '__APP_ID__'
$dest    = Join-Path $env:LOCALAPPDATA 'Programs\NorthviewOffice'
$src     = Split-Path -Parent $MyInvocation.MyCommand.Path
New-Item -ItemType Directory -Force -Path $dest | Out-Null
Get-Process -Name 'NorthviewOffice' -ErrorAction SilentlyContinue | Stop-Process -Force
Copy-Item -Force (Join-Path $src 'NorthviewOffice.exe') $dest
Copy-Item -Force (Join-Path $src 'icon.png') $dest
Copy-Item -Force (Join-Path $src 'uninstall.ps1') $dest
$exe = Join-Path $dest 'NorthviewOffice.exe'

# Notification identity (unpackaged app registration)
$key = "HKCU:\Software\Classes\AppUserModelId\$appId"
New-Item -Force -Path $key | Out-Null
Set-ItemProperty -Path $key -Name 'DisplayName' -Value 'Northview Office'
Set-ItemProperty -Path $key -Name 'IconUri' -Value (Join-Path $dest 'icon.png')

# Shortcuts
$shell = New-Object -ComObject WScript.Shell
foreach ($dir in @([Environment]::GetFolderPath('Desktop'), (Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'))) {
  $lnk = $shell.CreateShortcut((Join-Path $dir 'Northview Office.lnk'))
  $lnk.TargetPath = $exe
  $lnk.WorkingDirectory = $dest
  $lnk.IconLocation = "$exe,0"
  $lnk.Description = 'Northview Dental front-desk app'
  $lnk.Save()
}
Write-Host "Northview Office installed to $dest" -ForegroundColor Green
""".replace("__APP_ID__", APP_ID)

UNINSTALL_PS1 = r"""
$ErrorActionPreference = 'SilentlyContinue'
$appId = '__APP_ID__'
Get-Process -Name 'NorthviewOffice' | Stop-Process -Force
Remove-Item -Force (Join-Path ([Environment]::GetFolderPath('Desktop')) 'Northview Office.lnk')
Remove-Item -Force (Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Northview Office.lnk')
Remove-Item -Recurse -Force "HKCU:\Software\Classes\AppUserModelId\$appId"
Remove-Item -Recurse -Force (Join-Path $env:LOCALAPPDATA 'Programs\NorthviewOffice')
Write-Host 'Northview Office removed. Settings in %LOCALAPPDATA%\NorthviewOffice were kept.'
""".replace("__APP_ID__", APP_ID)

README = """Northview Office {version}

INSTALL (Windows)
  1. Unzip this folder.
  2. Right-click install.ps1 > Run with PowerShell.
  3. Open "Northview Office" from the Desktop.

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
    stage = dist / f"NorthviewOffice-{ver}"
    if stage.exists():
        shutil.rmtree(stage)
    stage.mkdir()
    shutil.copy2(ROOT / "assets" / "icon.png", stage / "icon.png")
    (stage / "README.txt").write_text(README.format(version=ver))

    if windows:
        shutil.copy2(exe, stage / "NorthviewOffice.exe")
        (stage / "install.ps1").write_text(INSTALL_PS1, encoding="utf-8-sig")
        (stage / "uninstall.ps1").write_text(UNINSTALL_PS1, encoding="utf-8-sig")
        out = dist / f"NorthviewOffice-{ver}-windows.zip"
        with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
            for f in sorted(stage.iterdir()):
                z.write(f, f"NorthviewOffice/{f.name}")
    else:
        shutil.copy2(exe, stage / "northview-office")
        out = dist / f"NorthviewOffice-{ver}-{platform.system().lower()}-{platform.machine()}.tar.gz"
        with tarfile.open(out, "w:gz") as t:
            t.add(stage, arcname="NorthviewOffice")
    shutil.rmtree(stage)
    print(f"Packaged {out.relative_to(ROOT)} ({out.stat().st_size // 1024} KB)")
    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Package a Linux executable, its installer, and a separate source archive."""

import argparse
import hashlib
import platform
import re
import shutil
import subprocess
import tarfile
import tempfile
import tomllib
from pathlib import Path

from build import APP, ROOT, binary_version, build_release
from notices import copy_notices

RUNTIME_FILES = (
    "install.py", "README.md", "README.uk.md", "docs", "scripts/build.py",
    "scripts/fetch-gg-presets.py", "packaging/linux",
)
SOURCE_FILES = (
    "Cargo.toml", "Cargo.lock", "AGENTS.md", "README.md", "README.uk.md",
    "Dockerfile", ".dockerignore", ".gitignore", ".github", "install", "install.py", "packaging", "docs",
    "scripts/build.py", "scripts/package.py", "scripts/notices.py", "scripts/render_icons.py", "scripts/fetch-gg-presets.py", "tests", "src",
)


def glibc_requirement(binary: Path) -> str:
    versions = subprocess.check_output(["readelf", "--version-info", str(binary)], text=True)
    requirements = {tuple(map(int, item.split("."))) for item in re.findall(r"GLIBC_([\d.]+)", versions)}
    return ".".join(map(str, max(requirements))) if requirements else "none detected"


def write_source(path: Path, name: str) -> None:
    with tarfile.open(path, "w:gz") as archive:
        for item in SOURCE_FILES:
            archive.add(ROOT / item, arcname=f"{name}/{item}", filter=source_entry)


def source_entry(info: tarfile.TarInfo) -> tarfile.TarInfo | None:
    if "__pycache__" in Path(info.name).parts:
        return None
    return info


def package(binary: Path, output: Path) -> None:
    version = binary_version(binary)
    with (ROOT / "Cargo.toml").open("rb") as manifest:
        if version != tomllib.load(manifest)["package"]["version"]:
            raise ValueError("The executable version does not match Cargo.toml.")
    name = f"{APP}-{version}"
    bundle_name = f"{name}-linux-{platform.machine()}"
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="nova-package-", dir=output) as tmp:
        stage = Path(tmp)
        bundle = stage / bundle_name
        (bundle / "bin").mkdir(parents=True)
        shutil.copy2(binary, bundle / "bin" / APP)
        subprocess.run(["strip", "--strip-debug", str(bundle / "bin" / APP)], check=True)
        for item in RUNTIME_FILES:
            destination = bundle / item
            destination.parent.mkdir(parents=True, exist_ok=True)
            if (ROOT / item).is_dir():
                shutil.copytree(ROOT / item, destination, ignore=shutil.ignore_patterns("__pycache__"))
            else:
                shutil.copy2(ROOT / item, destination)
        (bundle / "BUILD.txt").write_text(
            f"Nova Control {version}\nLinux {platform.machine()}\n"
            f"Minimum glibc symbols: {glibc_requirement(bundle / 'bin' / APP)}\n",
            encoding="utf-8",
        )
        copy_notices(bundle / "third-party")
        binary_archive = stage / f"{bundle_name}.tar.gz"
        source_archive = stage / f"{name}-source.tar.gz"
        with tarfile.open(binary_archive, "w:gz") as archive:
            archive.add(bundle, arcname=bundle_name)
        write_source(source_archive, name)
        standalone = stage / APP
        shutil.copy2(bundle / "bin" / APP, standalone)
        files = (binary_archive, source_archive, standalone)
        sums = stage / "SHA256SUMS"
        with sums.open("w", encoding="ascii") as checksums:
            for path in files:
                with path.open("rb") as data:
                    digest = hashlib.file_digest(data, "sha256").hexdigest()
                checksums.write(f"{digest}  {path.name}\n")
        for path in (*files, sums):
            destination = output / path.name
            path.replace(destination)
            print(destination)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, help="package an existing release build")
    parser.add_argument("--output", type=Path, default=ROOT / "dist", help="output directory")
    args = parser.parse_args()
    try:
        binary = args.binary.expanduser().resolve() if args.binary else build_release()
        package(binary, args.output.expanduser().resolve())
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Packaging failed: {error}\n")


if __name__ == "__main__":
    main()

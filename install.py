#!/usr/bin/env python3
"""Build or install Nova Control for the current Linux user."""

import argparse
import os
import shutil
import subprocess
import tempfile
from io import BytesIO
from pathlib import Path

from scripts.build import APP, ROOT, binary_version, build_release


def desktop_string(value: str) -> str:
    return value.replace("\\", "\\\\").replace("\n", "\\n").replace("\r", "\\r")


def desktop_exec(path: Path) -> str:
    command = str(path)
    # Escape for Exec's argument parser, then for the desktop entry parser.
    for char in ('\\', '"', '`', '$'):
        command = command.replace(char, '\\' + char)
    return desktop_string(f'"{command.replace("%", "%%")}"')


def desktop_data_home() -> Path:
    configured = Path(os.environ.get("XDG_DATA_HOME", ""))
    return configured if configured.is_absolute() else Path.home() / ".local/share"


def stage_file(destination: Path, data, mode: int) -> Path:
    destination.parent.mkdir(parents=True, exist_ok=True)
    file = tempfile.NamedTemporaryFile(
        dir=destination.parent, prefix=".nova-control-", suffix=destination.suffix, delete=False
    )
    staged = Path(file.name)
    try:
        with file:
            shutil.copyfileobj(data, file)
            os.fchmod(file.fileno(), mode)
    except BaseException:
        staged.unlink(missing_ok=True)
        raise
    return staged


def install(binary: Path, bin_dir: Path, data_home: Path) -> tuple[Path, Path]:
    command = bin_dir / APP
    desktop = data_home / "applications" / f"{APP}.desktop"
    template = (ROOT / "packaging/linux/nova-control.desktop.in").read_text(encoding="utf-8")
    entry = template.replace("@EXEC@", desktop_exec(command)).replace("@TRY_EXEC@", desktop_string(str(command)))

    with binary.open("rb") as data:
        new_command = stage_file(command, data, 0o755)
    new_entry = None
    try:
        new_entry = stage_file(desktop, BytesIO(entry.encode("utf-8")), 0o644)
        validator = shutil.which("desktop-file-validate")
        if validator:
            subprocess.run([validator, str(new_entry)], check=True)
        # Keep a working executable at a stable path across upgrades.
        os.replace(new_command, command)
        os.replace(new_entry, desktop)
    finally:
        new_command.unlink(missing_ok=True)
        if new_entry is not None:
            new_entry.unlink(missing_ok=True)
    return command, desktop


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, help="install an existing native build")
    parser.add_argument("--bin-dir", type=Path, default=Path.home() / ".local/bin",
                        help="installation directory for the command (default: ~/.local/bin)")
    args = parser.parse_args()
    try:
        if args.binary is not None:
            binary = args.binary.expanduser().resolve()
        elif (ROOT / "bin" / APP).is_file():
            binary = ROOT / "bin" / APP
        else:
            binary = build_release()
        version = binary_version(binary)
        command, desktop = install(binary, args.bin_dir.expanduser().absolute(), desktop_data_home())
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Installation failed: {error}\n")
    print(f"Nova Control {version}: {command}")
    print(f"Application menu: {desktop}")


if __name__ == "__main__":
    main()

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


def desktop_config_home() -> Path:
    configured = Path(os.environ.get("XDG_CONFIG_HOME", ""))
    return configured if configured.is_absolute() else Path.home() / ".config"


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


def install_icons(data_home: Path) -> None:
    source = ROOT / "packaging/linux/icons"
    if not source.is_dir():
        raise ValueError("The bundle is missing its application icons.")
    for icon in sorted(source.rglob("*")):
        if not icon.is_file():
            continue
        destination = data_home / "icons" / icon.relative_to(source)
        with icon.open("rb") as data:
            staged = stage_file(destination, data, 0o644)
        try:
            os.replace(staged, destination)
        finally:
            staged.unlink(missing_ok=True)


def desktop_entry(template: str, command: Path) -> BytesIO:
    text = (ROOT / "packaging/linux" / template).read_text(encoding="utf-8")
    entry = text.replace("@EXEC@", desktop_exec(command)).replace("@TRY_EXEC@", desktop_string(str(command)))
    return BytesIO(entry.encode("utf-8"))


def install(binary: Path, bin_dir: Path, data_home: Path, config_home: Path) -> tuple[Path, Path]:
    command = bin_dir / APP
    desktop = data_home / "applications" / f"{APP}.desktop"
    autostart = config_home / "autostart" / "nova-control-mixer.desktop"

    with binary.open("rb") as data:
        new_command = stage_file(command, data, 0o755)
    entries = {}
    try:
        entries[desktop] = stage_file(desktop, desktop_entry("nova-control.desktop.in", command), 0o644)
        entries[autostart] = stage_file(
            autostart, desktop_entry("nova-control-mixer.desktop.in", command), 0o644
        )
        validator = shutil.which("desktop-file-validate")
        if validator:
            for staged in entries.values():
                subprocess.run([validator, str(staged)], check=True)
        install_icons(data_home)
        # Keep a working executable at a stable path across upgrades.
        os.replace(new_command, command)
        for destination, staged in entries.items():
            os.replace(staged, destination)
    finally:
        new_command.unlink(missing_ok=True)
        for staged in entries.values():
            staged.unlink(missing_ok=True)
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
        command, desktop = install(binary, args.bin_dir.expanduser().absolute(), desktop_data_home(), desktop_config_home())
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Installation failed: {error}\n")
    print(f"Nova Control {version}: {command}")
    print(f"Application menu: {desktop}")


if __name__ == "__main__":
    main()

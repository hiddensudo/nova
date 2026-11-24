#!/usr/bin/env python3
"""Fetch the pinned community Sonar snapshot into Nova's local library."""
import argparse
import os
from pathlib import Path, PurePosixPath
import shutil
import tarfile
import tempfile
from urllib.request import urlopen

REVISION = "84545e49839e44137ce06b6e10df807ad0ad2440"
URL = f"https://codeload.github.com/loteran/Arctis-Sound-Manager/tar.gz/{REVISION}"
PREFIX = ("src", "arctis_sound_manager", "gui", "presets")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, help="use a previously downloaded snapshot")
    args = parser.parse_args()
    config = Path(os.environ.get("XDG_CONFIG_HOME", ""))
    if not config.is_absolute():
        config = Path.home() / ".config"
    library = config / "nova-control/presets/GG Sonar"
    library.mkdir(parents=True, exist_ok=True)

    with tempfile.TemporaryDirectory(prefix="nova-gg-") as temporary:
        staging = Path(temporary)
        if args.archive:
            source = args.archive
        else:
            source = staging / "snapshot.tar.gz"
            with urlopen(URL, timeout=45) as response, source.open("wb") as file:
                shutil.copyfileobj(response, file)
        files = []
        with tarfile.open(source, "r:gz") as archive:
            for member in archive:
                parts = PurePosixPath(member.name).parts
                if (
                    parts[1:-1] != PREFIX or not member.isfile()
                    or not 0 < member.size <= 256 * 1024
                    or not parts[-1].endswith(".json")
                ):
                    continue
                path = staging / parts[-1]
                with archive.extractfile(member) as data:
                    path.write_bytes(data.read())
                files.append(path)
        if not files:
            parser.error("No Sonar preset files were found in the snapshot.")
        for path in files:
            with tempfile.NamedTemporaryFile(dir=library, prefix=".import-", delete=False) as file:
                target = Path(file.name)
                file.write(path.read_bytes())
            try:
                target.replace(library / path.name)
            finally:
                target.unlink(missing_ok=True)
    print(f"Imported {len(files)} Sonar profiles into {library}")
    print("Open EQ in Nova Control and choose Reload library.")


if __name__ == "__main__":
    main()

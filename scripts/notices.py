"""Collect notices shipped by the Rust dependencies and bundled fonts."""

import json
import subprocess
from pathlib import Path

from build import ROOT


def copy_notices(destination: Path) -> None:
    compiler = subprocess.check_output(["rustc", "-vV"], text=True)
    host = next(line.split(": ", 1)[1] for line in compiler.splitlines() if line.startswith("host: "))
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--locked", "--format-version", "1", "--filter-platform", host],
        text=True, cwd=ROOT,
    ))
    resolved = {node["id"] for node in metadata["resolve"]["nodes"]}
    index = ["Third-party Rust packages and upstream license declarations", ""]
    for package in sorted(metadata["packages"], key=lambda item: (item["name"], item["version"])):
        if package["id"] not in resolved or package["id"] == metadata["resolve"]["root"]:
            continue
        folder = Path(package["manifest_path"]).parent
        name = f'{package["name"]}-{package["version"]}'
        index.append(f'{name}: {package["license"] or "see upstream notices"}')
        files = [path for path in folder.iterdir()
                 if path.is_file() and path.name.upper().startswith(("LICENSE", "COPYING", "NOTICE"))]
        if package["license_file"]:
            files.append(folder / package["license_file"])
        if package["name"] == "epaint_default_fonts":
            files.extend((folder / "fonts").glob("*.txt"))
        for path in set(files):
            if path.is_file():
                target = destination / name / path.name
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(path.read_bytes())
    destination.mkdir(parents=True, exist_ok=True)
    (destination / "README.txt").write_text("\n".join(index) + "\n", encoding="utf-8")

"""Locate the native Cargo build, including custom target directories."""

import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
APP = "nova-control"
VERSION_LINE = re.compile(r"nova-control (\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.+-]+)?)")


def build_release() -> Path:
    result = subprocess.run(
        ["cargo", "build", "--release", "--locked", "--message-format=json-render-diagnostics"],
        cwd=ROOT,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    )
    for line in result.stdout.splitlines():
        message = json.loads(line)
        if message.get("executable") and message["target"]["name"] == APP:
            return Path(message["executable"])
    raise ValueError("Cargo did not produce the Nova Control executable.")


def binary_version(binary: Path) -> str:
    result = subprocess.run(
        [str(binary), "--version"], check=True, text=True, stdout=subprocess.PIPE
    )
    match = VERSION_LINE.fullmatch(result.stdout.strip())
    if match is None:
        raise ValueError(f"Unexpected version output from {binary}.")
    return match[1]

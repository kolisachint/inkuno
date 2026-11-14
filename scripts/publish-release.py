#!/usr/bin/env python3
"""Publish only a complete, nonempty set of artifacts from the current workflow run."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib


def expected_assets(root: Path, tag: str) -> list[str]:
    if not re.fullmatch(r"v[0-9]+\.[0-9]+\.[0-9]+", tag):
        raise ValueError("tag must look like v1.2.3")
    package = tomllib.loads((root / "crates/inkuno/Cargo.toml").read_text())["package"]
    app = package["name"]
    version = tag[1:]
    if package["version"] != version:
        raise ValueError("tag and crates/inkuno/Cargo.toml versions differ")
    if app not in {"inkuno", "spot", "astrolabe"}:
        raise ValueError(f"unsupported application: {app}")
    # macOS + Linux only. Windows installers/zips and Arch packages were
    # dropped from the pipeline; the rename crew owns the remaining `bref`
    # asset strings, so names follow crates/inkuno/Cargo.toml instead of hardcoding them.
    names = [f"{app}-macos.dmg", f"{app}-linux-x86_64.tar.gz"]
    return sorted(names)


def validate_files(root: Path, tag: str, directory: Path) -> list[Path]:
    names = expected_assets(root, tag)
    actual = {path.name for path in directory.iterdir()}
    if actual - {"SHA256SUMS"} != set(names):
        raise ValueError(f"artifact mismatch: expected {names}, received {sorted(actual)}")
    paths = [directory / name for name in names]
    for path in paths:
        if path.is_symlink() or not path.is_file() or path.stat().st_size == 0:
            raise ValueError(f"artifact must be a nonempty regular file: {path.name}")
    return paths


def gh(*args: str):
    return subprocess.run(["gh", *args], check=True, text=True, capture_output=True)


def draft(tag: str) -> dict:
    release = json.loads(gh("release", "view", tag, "--json", "isDraft,assets").stdout)
    if release.get("isDraft") is not True:
        raise ValueError("refusing to modify a published release; a draft is required")
    return release


def publish(root: Path, tag: str, directory: Path) -> None:
    paths = validate_files(root, tag, directory)
    draft(tag)
    sums = directory / "SHA256SUMS"
    if sums.is_symlink():
        raise ValueError("SHA256SUMS must not be a symlink")
    # Stream packages rather than holding an entire installer in memory.
    with sums.open("w") as output:
        for path in paths:
            with path.open("rb") as source:
                digest = hashlib.file_digest(source, "sha256").hexdigest()
            output.write(f"{digest}  {path.name}\n")
    paths.append(sums)
    gh("release", "upload", tag, *(str(path) for path in paths), "--clobber")
    release = draft(tag)
    remote = {asset["name"]: asset["size"] for asset in release["assets"]}
    expected = {path.name: path.stat().st_size for path in paths}
    if remote != expected:
        raise ValueError(f"uploaded assets do not match the complete local build: {remote}")
    gh("release", "edit", tag, "--draft=false", "--latest", "--verify-tag")
    print(f"Published {tag} with {len(paths)} verified assets.")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("tag")
    parser.add_argument("directory", type=Path)
    parser.add_argument("--check-only", action="store_true", help="check local artifacts without contacting GitHub")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    try:
        if args.check_only:
            validate_files(root, args.tag, args.directory)
        else:
            publish(root, args.tag, args.directory)
    except (ValueError, OSError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        parser.exit(1, f"release: {error}\n")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Generate the proto registry dataset for moonrepo/community-plugins."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import tomllib
from typing import Any


COMMUNITY_REPOSITORY = "https://github.com/moonrepo/community-plugins"
COMMUNITY_RAW_ROOT = (
    "https://raw.githubusercontent.com/moonrepo/community-plugins/master/tools"
)


def load_manifest(path: Path) -> dict[str, Any]:
    try:
        with path.open("rb") as file:
            manifest = tomllib.load(file)
    except tomllib.TOMLDecodeError as error:
        raise ValueError(f"failed to parse {path}: {error}") from error

    if not isinstance(manifest, dict):
        raise ValueError(f"expected {path} to contain a TOML table")

    return manifest


def require_table(parent: dict[str, Any], key: str, path: Path) -> dict[str, Any]:
    value = parent.get(key, {})

    if not isinstance(value, dict):
        raise ValueError(f"expected [{key}] in {path} to be a table")

    return value


def require_string(parent: dict[str, Any], key: str, path: Path) -> str:
    value = parent.get(key)

    if not isinstance(value, str) or not value:
        raise ValueError(f"expected {key!r} in {path} to be a non-empty string")

    return value


def string_list(parent: dict[str, Any], key: str, path: Path) -> list[str]:
    value = parent.get(key, [])

    if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
        raise ValueError(f"expected {key!r} in {path} to be an array of strings")

    return value


def parse_bins(plugin_id: str, install: dict[str, Any], path: Path) -> list[str]:
    exes = install.get("exes", {})

    if not isinstance(exes, dict):
        raise ValueError(f"expected [install.exes] in {path} to be a table")

    primary_bins: list[str] = []
    secondary_bins: list[str] = []

    for bin_name, config in exes.items():
        if not isinstance(config, dict):
            raise ValueError(
                f"expected [install.exes.{bin_name}] in {path} to be a table"
            )

        if config.get("primary") is True:
            primary_bins.append(bin_name)
        else:
            secondary_bins.append(bin_name)

    # The TOML plugin implicitly registers the plugin ID as the primary executable
    # when none of the explicitly configured executables are marked as primary.
    if not primary_bins:
        primary_bins.append(plugin_id)

    return list(dict.fromkeys([*primary_bins, *secondary_bins]))


def build_plugin_entry(path: Path) -> dict[str, Any]:
    manifest = load_manifest(path)
    plugin_id = path.stem
    metadata = require_table(manifest, "plugin", path)
    install = require_table(manifest, "install", path)
    detect = require_table(manifest, "detect", path)
    packages = require_table(manifest, "packages", path)

    entry: dict[str, Any] = {
        "id": plugin_id,
        "locator": f"{COMMUNITY_RAW_ROOT}/{path.name}",
        "format": "toml",
        "name": require_string(manifest, "name", path),
        "description": metadata.get("description", ""),
        "author": "moonrepo",
    }

    if not isinstance(entry["description"], str):
        raise ValueError(f"expected 'plugin.description' in {path} to be a string")

    for source_key, output_key in (
        ("homepage-url", "homepageUrl"),
        ("repository-url", "repositoryUrl"),
    ):
        value = metadata.get(source_key)

        if value is not None:
            if not isinstance(value, str):
                raise ValueError(f"expected 'plugin.{source_key}' in {path} to be a string")

            entry[output_key] = value

    entry.setdefault("repositoryUrl", COMMUNITY_REPOSITORY)
    entry["bins"] = parse_bins(plugin_id, install, path)

    version_files = string_list(detect, "version-files", path)
    if version_files:
        entry["detectionSources"] = [{"file": file} for file in version_files]

    globals_dirs = string_list(packages, "globals-lookup-dirs", path)
    if globals_dirs:
        entry["globalsDirs"] = globals_dirs

    return entry


def generate_registry(source_root: Path) -> dict[str, Any]:
    tools_dir = source_root / "tools"

    if not tools_dir.is_dir():
        raise ValueError(f"community plugins tools directory does not exist: {tools_dir}")

    manifests = sorted(tools_dir.glob("*.toml"), key=lambda path: path.name)
    if not manifests:
        raise ValueError(f"no community plugin manifests found in {tools_dir}")

    return {
        "$schema": "../schema.json",
        "version": 1,
        "plugins": [build_plugin_entry(path) for path in manifests],
    }


def write_registry(source_root: Path, output_path: Path) -> None:
    registry = generate_registry(source_root)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(registry, indent=2), encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "source_root",
        type=Path,
        help="path to a checkout of moonrepo/community-plugins",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("registry/data/community.json"),
        help="registry JSON file to write",
    )
    args = parser.parse_args()

    write_registry(args.source_root, args.output)


if __name__ == "__main__":
    main()

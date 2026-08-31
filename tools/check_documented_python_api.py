#!/usr/bin/env python3
"""Fail when an exported Python API name has no reference-page heading."""

from __future__ import annotations

import ast
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCES = (
    ROOT / "python" / "rhealpixdggs" / "__init__.py",
    ROOT / "python" / "rhealpixdggs" / "numpy.py",
    ROOT / "python" / "rhealpixdggs" / "geo.py",
)
MODULE_EXPORTS = {"geo", "numpy"}
HEADING = re.compile(r"^#{2,4}\s+`([^`]+)`\s*$", re.MULTILINE)


def exported_names(path: Path) -> set[str]:
    tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
    for node in tree.body:
        if not isinstance(node, (ast.Assign, ast.AnnAssign)):
            continue
        targets = node.targets if isinstance(node, ast.Assign) else [node.target]
        if not any(
            isinstance(target, ast.Name) and target.id == "__all__"
            for target in targets
        ):
            continue
        value = ast.literal_eval(node.value)
        return {str(name) for name in value}
    raise ValueError(f"no literal __all__ found in {path}")


def documented_names() -> set[str]:
    result: set[str] = set()
    for path in (ROOT / "docs" / "api").glob("*.md"):
        for heading in HEADING.findall(path.read_text(encoding="utf-8")):
            result.add(heading.rsplit(".", 1)[-1])
    return result


def main() -> None:
    exports = set().union(*(exported_names(path) for path in SOURCES))
    required = exports - MODULE_EXPORTS
    missing = sorted(required - documented_names())
    if missing:
        names = ", ".join(missing)
        raise SystemExit(f"undocumented public Python API names: {names}")
    print(f"verified reference headings for {len(required)} public Python names")


if __name__ == "__main__":
    main()


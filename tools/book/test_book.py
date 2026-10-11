"""Runs the code examples in docs/book.

`mdbook test` cannot choose between several builds of the same crate in target/debug/deps. This
script asks Cargo for the rlibs of the tpt-colour build graph, copies just those into a clean
directory, and points mdbook at it:

    python tools/book/test_book.py
"""

import json
import pathlib
import shutil
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
DEPS = ROOT / "target" / "debug" / "deps"
STAGE = ROOT / "target" / "book-deps"


def graph_rlibs() -> list[pathlib.Path]:
    """The rlibs of the exact build graph of tpt-colour, as Cargo reports them."""
    out = subprocess.run(
        ["cargo", "build", "-p", "tpt-colour", "--message-format=json"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    paths = []
    for line in out.splitlines():
        message = json.loads(line) if line.startswith("{") else {}
        if message.get("reason") != "compiler-artifact":
            continue
        for filename in message.get("filenames", []):
            if filename.endswith(".rlib"):
                paths.append(pathlib.Path(filename))
    return paths


def main() -> int:
    if STAGE.exists():
        shutil.rmtree(STAGE)
    STAGE.mkdir(parents=True)
    rlibs = graph_rlibs()
    if not any(path.name.startswith("libtpt_colour") for path in rlibs):
        print("no tpt-colour rlibs reported by cargo", file=sys.stderr)
        return 1
    for path in rlibs:
        shutil.copy2(path, STAGE / path.name)
    result = subprocess.run(
        ["mdbook", "test", str(ROOT / "docs" / "book"), "-L", str(STAGE)],
        check=False,
    )
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())

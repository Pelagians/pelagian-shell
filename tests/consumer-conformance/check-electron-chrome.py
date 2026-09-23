#!/usr/bin/env python3
"""Early static gate for known Electron window construction sites."""
import argparse
from pathlib import Path
import re
import sys

CALL = re.compile(r"\bnew\s+BrowserWindow\s*\(")
EXCEPTION = re.compile(
    r"pelagian-shell-chrome-exception:\s*(overlay|hud|splash|tooltip|menu|notification|utility)\s+--\s+([^\n]{10,})"
)


def _matching_paren(source: str, opening: int) -> int:
    stack = [")"]
    quote = None
    escaped = False
    line_comment = False
    block_comment = False
    index = opening + 1
    pairs = {"(": ")", "{": "}", "[": "]"}
    while index < len(source):
        char = source[index]
        next_char = source[index + 1] if index + 1 < len(source) else ""
        if line_comment:
            if char == "\n":
                line_comment = False
        elif block_comment:
            if char == "*" and next_char == "/":
                block_comment = False
                index += 1
        elif quote:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == quote:
                quote = None
        elif char == "/" and next_char == "/":
            line_comment = True
            index += 1
        elif char == "/" and next_char == "*":
            block_comment = True
            index += 1
        elif char in ("'", '"', chr(96)):
            quote = char
        elif char in pairs:
            stack.append(pairs[char])
        elif char in (")", "}", "]"):
            if not stack or char != stack.pop():
                raise ValueError(f"unbalanced JavaScript delimiter at byte {index}")
            if not stack:
                return index
        index += 1
    raise ValueError("unterminated BrowserWindow constructor")


def inspect_source(source: str) -> list[str]:
    violations = []
    previous_end = 0
    for match in CALL.finditer(source):
        opening = source.find("(", match.start(), match.end())
        try:
            closing = _matching_paren(source, opening)
        except ValueError as error:
            violations.append(str(error))
            continue
        arguments = source[opening + 1 : closing]
        line = source.count("\n", 0, match.start()) + 1
        nearby = source[max(previous_end, match.start() - 500) : match.start()]
        exception = list(EXCEPTION.finditer(nearby))
        adapted = "applyPelagianShellWindowChrome(" in arguments
        if adapted or exception:
            previous_end = closing + 1
            continue
        if re.search(r"\b(?:frame\s*:\s*false|titleBarStyle\s*:|titleBarOverlay\s*:)", arguments):
            reason = "raw client chrome option bypasses the Shell adapter"
        else:
            previous_end = closing + 1
            continue
        violations.append(f"line {line}: {reason}")
        previous_end = closing + 1
    return violations


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("sources", nargs="+", type=Path)
    args = parser.parse_args()
    failed = False
    for path in args.sources:
        violations = inspect_source(path.read_text(encoding="utf-8"))
        for violation in violations:
            print(f"{path}:{violation}", file=sys.stderr)
        failed |= bool(violations)
    if not failed:
        print("Electron window chrome: no explicit client chrome bypasses the Shell adapter")
    return int(failed)


if __name__ == "__main__":
    raise SystemExit(main())

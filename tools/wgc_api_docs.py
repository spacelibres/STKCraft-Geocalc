#!/usr/bin/env python3
"""Convert the WGC API HTML documentation to readable text."""
from __future__ import annotations

from html.parser import HTMLParser
from pathlib import Path


class Extractor(HTMLParser):
    def __init__(self) -> None:
        super().__init__()
        self.parts: list[str] = []
        self.skip = 0

    def handle_starttag(self, tag: str, attrs) -> None:
        if tag in ("script", "style"):
            self.skip += 1
        if self.skip:
            return
        if tag in ("h1", "h2", "h3", "h4", "h5", "h6", "p", "tr", "li", "pre", "br", "div"):
            self.parts.append("\n")
        if tag in ("td", "th"):
            self.parts.append(" | ")

    def handle_endtag(self, tag: str) -> None:
        if tag in ("script", "style"):
            self.skip = max(0, self.skip - 1)
        if self.skip:
            return
        if tag in ("h1", "h2", "h3", "h4", "h5", "h6", "p", "tr", "li", "pre", "div"):
            self.parts.append("\n")

    def handle_data(self, data: str) -> None:
        if not self.skip:
            self.parts.append(data)


def main() -> int:
    source = Path("target/wgc/api-info.html")
    target = Path("target/wgc/api-info.txt")
    parser = Extractor()
    parser.feed(source.read_text(encoding="utf-8", errors="replace"))
    text = "".join(parser.parts)
    lines = [line.rstrip() for line in text.splitlines()]
    cleaned: list[str] = []
    blank = 0
    for line in lines:
        if not line.strip():
            blank += 1
            if blank > 1:
                continue
        else:
            blank = 0
        cleaned.append(line)
    target.write_text("\n".join(cleaned), encoding="utf-8", newline="\n")
    print(f"wrote {target} ({len(cleaned)} lines)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

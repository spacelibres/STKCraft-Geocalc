#!/usr/bin/env python3
"""Extract output IDs and parameter names from WGC API doc sections."""
from __future__ import annotations

import json
import re
from pathlib import Path

SECTIONS = Path("target/wgc/sections")
for path in sorted(SECTIONS.glob("*.txt")):
    if path.name == "summary.csv":
        continue
    text = path.read_text(encoding="utf-8")
    outputs = re.findall(r'"outputID"\s*:\s*"([^"]+)"', text)
    # unique preserving order
    seen = []
    for output in outputs:
        if output not in seen:
            seen.append(output)
    print(f"{path.stem}: {', '.join(seen)}")

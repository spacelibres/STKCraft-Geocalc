#!/usr/bin/env python3
"""Split the WGC API documentation into per-calculation sections."""
from __future__ import annotations

import re
from pathlib import Path

TYPES = [
    "STATE_VECTOR",
    "ANGULAR_SEPARATION",
    "ANGULAR_SIZE",
    "FRAME_TRANSFORMATION",
    "ILLUMINATION_ANGLES",
    "PHASE_ANGLE",
    "PLANETOCENTRIC_SOLAR_LONGITUDE",
    "POINTING_DIRECTION",
    "SUB_SOLAR_POINT",
    "SUB_OBSERVER_POINT",
    "SURFACE_INTERCEPT_POINT",
    "TANGENT_POINT",
    "OSCULATING_ELEMENTS",
    "GF_COORDINATE_SEARCH",
    "GF_ANGULAR_SEPARATION_SEARCH",
    "GF_DISTANCE_SEARCH",
    "GF_SUB_POINT_SEARCH",
    "GF_OCCULTATION_SEARCH",
    "GF_SURFACE_INTERCEPT_POINT_SEARCH",
    "GF_TARGET_IN_INSTRUMENT_FOV_SEARCH",
    "GF_RAY_IN_FOV_SEARCH",
    "GF_RANGE_RATE_SEARCH",
    "GF_PHASE_ANGLE_SEARCH",
    "GF_ILLUMINATION_ANGLES_SEARCH",
    "TIME_CONVERSION",
]


def main() -> int:
    text = Path("target/wgc/api-info.txt").read_text(encoding="utf-8")
    lines = text.splitlines()
    starts = {}
    for index, line in enumerate(lines):
        stripped = line.strip()
        if stripped in TYPES and stripped not in starts and index > 2300:
            starts[stripped] = index
    ordered = sorted(starts.items(), key=lambda item: item[1])
    out_dir = Path("target/wgc/sections")
    out_dir.mkdir(parents=True, exist_ok=True)
    summary = []
    for position, (name, start) in enumerate(ordered):
        end = ordered[position + 1][1] if position + 1 < len(ordered) else len(lines)
        body = lines[start:end]
        (out_dir / f"{name}.txt").write_text("\n".join(body), encoding="utf-8", newline="\n")
        parameters = []
        for line in body:
            match = re.match(r"\s*\|\s*([A-Za-z][A-Za-z0-9_]*)\s*\|", line)
            if match:
                parameters.append(match.group(1))
        summary.append((name, len(body), ",".join(parameters)))
    with Path("target/wgc/sections/summary.csv").open("w", encoding="utf-8", newline="\n") as handle:
        handle.write("type,lines,parameters\n")
        for row in summary:
            handle.write(f"{row[0]},{row[1]},\"{row[2]}\"\n")
    for row in summary:
        print(f"{row[0]:45s} {row[1]:5d}  {row[2]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

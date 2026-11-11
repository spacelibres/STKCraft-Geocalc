#!/usr/bin/env python3
"""Probe WGC State Vector output for every state representation."""
from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import wgc2_geometry_compare as m

wgc = m.Wgc(True, 120.0)

CASES = [
    ("J2000", "RECTANGULAR", None, None),
    ("J2000", "RA_DEC", None, None),
    ("J2000", "LATITUDINAL", None, None),
    ("J2000", "CYLINDRICAL", None, None),
    ("J2000", "SPHERICAL", None, None),
    ("J2000", "AZ_EL", True, True),
    ("IAU_MOON", "PLANETODETIC", None, None),
    ("IAU_MOON", "PLANETOGRAPHIC", None, None),
]

for frame, representation, azccw, elplsz in CASES:
    payload = {
        "calculationType": "STATE_VECTOR",
        "kernels": [{"type": "KERNEL_SET", "id": 1}],
        "target": "MOON",
        "observer": "EARTH",
        "referenceFrame": frame,
        "aberrationCorrection": "NONE",
        "stateRepresentation": representation,
        **m.time_block([0.0]),
    }
    if azccw is not None:
        payload["azccwFlag"] = azccw
        payload["elplszFlag"] = elplsz
    try:
        result = wgc.run(payload)
        columns = [(column["name"], column["outputID"]) for column in result["columns"]]
        print(f"=== {frame} / {representation} ===")
        print(json.dumps(columns, indent=2))
        print(result["rows"][0])
        print()
    except Exception as error:
        print(f"=== {frame} / {representation} === ERROR {error}")

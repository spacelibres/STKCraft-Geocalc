#!/usr/bin/env python3
"""Probe WGC AZ_EL flag semantics."""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import wgc2_geometry_compare as m

wgc = m.Wgc(True, 120.0)
for azccw in (True, False):
    for elplsz in (True, False):
        payload = {
            "calculationType": "STATE_VECTOR",
            "kernels": [{"type": "KERNEL_SET", "id": 1}],
            "target": "MOON",
            "observer": "EARTH",
            "referenceFrame": "J2000",
            "aberrationCorrection": "NONE",
            "stateRepresentation": "AZ_EL",
            "azccwFlag": azccw,
            "elplszFlag": elplsz,
            **m.time_block([0.0]),
        }
        result = wgc.run(payload)
        row = result["rows"][0]
        print(f"azccw={azccw} elplsz={elplsz}: az={row[1]} el={row[2]} range={row[3]}")

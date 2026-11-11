#!/usr/bin/env python3
"""Generate WebGeocalc planetocentric solar longitude golden data.

Queries the live WGC2 `PLANETOCENTRIC_SOLAR_LONGITUDE` calculation and
writes the results. `tests/surface_golden.rs` compares the Rust
implementation.

Usage:
    python tools/gen_solar_longitude_golden.py
"""

from __future__ import annotations

import argparse
import json
import ssl
import time
import urllib.request
from pathlib import Path

API_BASE = "https://wgc2.jpl.nasa.gov:8443/webgeocalc/api"
EPOCHS = [-5.0e8, 0.0, 5.0e8]
BODIES = ["MOON", "EARTH"]


def request(method: str, path: str, payload: dict | None, context) -> dict:
    data = None if payload is None else json.dumps(payload).encode()
    req = urllib.request.Request(
        f"{API_BASE}{path}",
        data=data,
        headers={"Content-Type": "application/json", "Accept": "application/json"},
        method=method,
    )
    with urllib.request.urlopen(req, context=context, timeout=60) as response:
        return json.loads(response.read().decode())


def run(payload: dict, context) -> dict:
    created = request("POST", "/calculation/new", payload, context)
    if created.get("status") != "OK":
        raise RuntimeError(json.dumps(created)[:400])
    calculation_id = created["calculationId"]
    for _ in range(120):
        status = request("GET", f"/calculation/{calculation_id}", None, context)
        if status.get("result", {}).get("phase") == "COMPLETE":
            return request(
                "GET", f"/calculation/{calculation_id}/results", None, context
            )
        if status.get("result", {}).get("phase") in ("FAILED", "CANCELLED", "EXPIRED"):
            raise RuntimeError(json.dumps(status)[:400])
        time.sleep(0.4)
    raise TimeoutError(calculation_id)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", default="tests/golden/solar_longitude.csv")
    parser.add_argument("--insecure", action="store_true")
    args = parser.parse_args()

    context = (
        ssl._create_unverified_context()
        if args.insecure
        else ssl.create_default_context()
    )

    rows = []
    for body in BODIES:
        frame = f"IAU_{body}"
        results = run(
            {
                "calculationType": "PLANETOCENTRIC_SOLAR_LONGITUDE",
                "kernels": [{"type": "KERNEL_SET", "id": 1}],
                "body": body,
                "targetFrame": frame,
                "aberrationCorrection": "NONE",
                "timeSystem": "TDB",
                "timeFormat": "SECONDS_PAST_J2000",
                "times": [repr(et) for et in EPOCHS],
                "outputTimeSystem": "TDB",
                "outputTimeFormat": "SECONDS_PAST_J2000",
            },
            context,
        )
        for et, row in zip(EPOCHS, results["rows"]):
            rows.append((body, et, float(row[1])))

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write("# WGC2 PLANETOCENTRIC_SOLAR_LONGITUDE, degrees.\n")
        handle.write("body,et,longitude_deg\n")
        for body, et, value in rows:
            handle.write(f"{body},{et!r},{value!r}\n")

    print(f"wrote {out} ({len(rows)} rows)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

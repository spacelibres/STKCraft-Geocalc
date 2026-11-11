#!/usr/bin/env python3
"""Generate golden data for every WebGeocalc ray-direction type.

CSPICE `sincpt`/`tangpt` take a plain J2000 direction vector, so the
direction-type resolution WebGeocalc performs above them has no CSPICE
oracle. This script therefore resolves each specification independently in
Python -- with `pxform` for frame vectors and `spkezp` for object
directions -- records both the resolved J2000 vector and the CSPICE
intercept/tangent point computed from it. `tests/ray_direction_golden.rs`
then resolves the same specification with this crate and compares both.

The synthetic kernel `tests/golden/fov_ellipsoid.tf` already contains
frames whose `+Z` axis points at the Moon as seen from the Earth, plus
instruments of all four FOV shapes, so `REFERENCE_FRAME_AXIS`,
`INSTRUMENT_BORESIGHT`, `VECTOR_IN_INSTRUMENT_FOV` and
`INSTRUMENT_FOV_BOUNDARY_VECTORS` all have intercepts to find.

Offline tooling only; needs `spiceypy`.

Usage:
    python tools/gen_ray_direction_golden.py \
        --spk target/kernels/de440s.bsp --pck target/kernels/pck00011.tpc
"""

from __future__ import annotations

import argparse
from pathlib import Path

import spiceypy as spice

FOV_KERNEL = "tests/golden/fov_ellipsoid.tf"

# instrument ids in fov_ellipsoid.tf: 1420001.. are CIRCLE/ELLIPSE/
# RECTANGLE/POLYGON for the first frame, which points at the Moon.
EARTH = 399
MOON = 301
EPOCHS = [-1.0e8, 0.0, 1.0e8]


def frame_of(instrument: int) -> str:
    text = Path(FOV_KERNEL).read_text(encoding="utf-8")
    for line in text.splitlines():
        if line.strip().startswith(f"INS{instrument}_FOV_FRAME"):
            return line.split("'")[1]
    raise LookupError(f"instrument {instrument} has no FOV_FRAME")


def resolve(spec: dict, observer: int, et: float) -> list[float]:
    """Resolve one specification to a J2000 direction vector."""

    kind = spec["kind"]
    if kind == "OBJECT":
        position, _ = spice.spkezp(int(spec["object"]), et, "J2000", "NONE", observer)
        vector = [float(value) for value in position]
        return [-value for value in vector] if spec["anti"] else vector

    if kind == "AXIS":
        axis = {"X": [1.0, 0.0, 0.0], "Y": [0.0, 1.0, 0.0], "Z": [0.0, 0.0, 1.0]}
        rotation = spice.pxform(spec["frame"], "J2000", et)
        vector = spice.mxv(rotation,axis[spec["axis"]]).tolist()
        return [-value for value in vector] if spec["anti"] else vector

    if kind == "VECTOR":
        rotation = spice.pxform(spec["frame"], "J2000", et)
        raw = spec["vector"]
        vector = spice.mxv(rotation,raw).tolist()
        return [-value for value in vector] if spec["anti"] else vector

    if kind == "BORESIGHT" or kind == "VECFOV":
        instrument = int(spec["instrument"])
        frame = frame_of(instrument)
        if kind == "BORESIGHT":
            raw = [float(value) for value in spice.getfov(instrument, 10)[2]]
        else:
            raw = spec["vector"]
        rotation = spice.pxform(frame, "J2000", et)
        return spice.mxv(rotation,raw).tolist()

    if kind == "FOVBOUND":
        raise ValueError("FOVBOUND is expanded by the caller")

    raise ValueError(f"unknown kind {kind}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--spk", default="target/kernels/de440s.bsp")
    parser.add_argument("--pck", default="target/kernels/pck00011.tpc")
    parser.add_argument("--out", default="tests/golden/ray_directions.csv")
    args = parser.parse_args()

    spice.furnsh(args.spk)
    spice.furnsh(args.pck)
    spice.furnsh(FOV_KERNEL)

    frame = frame_of(1420001)
    specs = [
        {"kind": "OBJECT", "object": str(MOON), "anti": False},
        {"kind": "OBJECT", "object": str(MOON), "anti": True},
        {"kind": "AXIS", "frame": frame, "axis": "Z", "anti": False},
        {"kind": "AXIS", "frame": frame, "axis": "Z", "anti": True},
        {"kind": "AXIS", "frame": "ECLIPJ2000", "axis": "X", "anti": False},
        {"kind": "VECTOR", "frame": frame, "vector": [0.02, -0.01, 1.0], "anti": False},
        {"kind": "VECTOR", "frame": "J2000", "vector": [1.0, 0.5, 0.25], "anti": False},
        {"kind": "BORESIGHT", "instrument": "1420001"},
        {"kind": "BORESIGHT", "instrument": "1420003"},
        {"kind": "VECFOV", "instrument": "1420002", "vector": [0.01, 0.01, 1.0]},
    ]

    rows = []
    for et in EPOCHS:
        for spec in specs:
            vectors = [resolve(spec, EARTH, et)]
            rows.append((et, spec, "", vectors[0]))

        # Expand the FOV boundary vectors of a rectangle.
        instrument = 1420003
        frame_name = frame_of(instrument)
        boundary = spice.getfov(instrument, 10)[4]
        rotation = spice.pxform(frame_name, "J2000", et)
        for index, raw in enumerate(boundary):
            vector = spice.mxv(rotation,raw).tolist()
            rows.append(
                (et, {"kind": "FOVBOUND", "instrument": str(instrument)}, str(index), vector)
            )

    results = []
    for et, spec, index, vector in rows:
        for method in ("sincpt", "tangpt"):
            try:
                if method == "sincpt":
                    point, _, _ = spice.sincpt(
                        "ELLIPSOID", str(MOON), et, "IAU_MOON", "NONE", str(EARTH),
                        "J2000", vector,
                    )
                    extra = 0.0
                    found = 1
                else:
                    tanpt, alt, rng, srfpt, _, _ = spice.tangpt(
                        "ELLIPSOID", str(MOON), et, "IAU_MOON", "NONE", "TANGENT POINT",
                        str(EARTH), "J2000", vector,
                    )
                    point = tanpt
                    extra = float(rng)
                    found = 1
            except Exception:
                point = [0.0, 0.0, 0.0]
                extra = 0.0
                found = 0
            results.append((method, et, spec, index, vector, point, extra, found))

    spice.kclear()

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(
            "# Ray-direction resolution and the CSPICE result computed from it.\n"
            "# `dx,dy,dz` is the resolved J2000 direction; `px,py,pz` the point\n"
            "# in IAU_MOON; `extra` is the tangent-point range (km);\n"
            "# `found` is 0 when CSPICE reported no point.\n"
        )
        handle.write(
            "method,et,speckind,s1,s2,s3,vx,vy,vz,dx,dy,dz,px,py,pz,extra,found\n"
        )
        for method, et, spec, index, vector, point, extra, found in results:
            kind = spec["kind"]
            s1 = s2 = s3 = ""
            vx = vy = vz = 0.0
            if kind == "OBJECT":
                s1, s2 = str(spec["object"]), str(spec["anti"]).lower()
            elif kind == "AXIS":
                s1, s2, s3 = spec["frame"], spec["axis"], str(spec["anti"]).lower()
            elif kind in ("VECTOR", "VECFOV"):
                if kind == "VECTOR":
                    s1, s2 = spec["frame"], str(spec["anti"]).lower()
                    s3 = str(spec.get("stellar", False)).lower()
                else:
                    s1 = str(spec["instrument"])
                vx, vy, vz = (float(value) for value in spec["vector"])
            elif kind == "BORESIGHT":
                s1 = str(spec["instrument"])
            elif kind == "FOVBOUND":
                s1, s2 = str(spec["instrument"]), index
            numbers = [float(value) for value in list(vector) + list(point)] + [float(extra)]
            text = ",".join(repr(value) for value in numbers)
            handle.write(
                f"{method},{et!r},{kind},{s1},{s2},{s3},{vx!r},{vy!r},{vz!r},{text},{found}\n"
            )

    print(f"wrote {out} ({len(results)} rows)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

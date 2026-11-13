# NASA calculator observations

The JSONL files retain the request and the raw response body for each WGC2
HTTP operation captured on 2026-11-13 (Asia/Shanghai). The kernel selection
is server-managed set 1; `angular-size-kernelsets.jsonl` also compares set 1
with sets 1 and 3. Server kernel bytes were not independently retrieved.
`SHA256SUMS.txt` protects the retained JSONL and native-capture JSON bytes.

The local replay uses DE442 and pck00011; its report records their hashes.
It compares every covered output to the previously established WGC decimal
serialization rule, without a numerical tolerance. This is evidence about the
displayed WGC values; independently captured native CSPICE tests are needed
for claims about unrounded binary64 results. Two Sub-observer Point emission
angles and two Tangent Point altitudes are currently unequal and deliberately
make the replay fail. The angular-size grid and matrix, and the sub-observer
local-time grid and matrix, retain additional inputs for these investigations.
`angular-size-triaxial-mars.jsonl` adds Phobos, Deimos, and Mars observations;
the local replay uses NAIF `mar099s.bsp` in addition to DE442. Its kernel
selection is not proof that server-managed kernel bytes are identical.
`angular-size-native-windows.json` and `angular-size-native-linux.json` record
the same three EDLIMB/ZZASRYEL cases against separate CSPICE N0067 builds;
their hexadecimal native outputs match. Reproduce with
`python tools/capture_angular_size_native.py --library <CSPICE-library>
--output <output.json>` and the pinned local kernels.
`angular-size-error-path.jsonl` records that a coincident observer causes
WGC's LIMB_EXTENTS request to report `CSPICE.edlimb` and
`SPICE(DEGENERATECASE)`; it does not identify the full WGC algorithm.
`edlimb-errors-native.json` independently records the CSPICE C wrapper's
error symbol for an interior observer and three invalid or degenerate radii.
`angular-size-earth-frames-native.json` records the native limb widths used
by the 18-case explicit-frame binary64 regression.

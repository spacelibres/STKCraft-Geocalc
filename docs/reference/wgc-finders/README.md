# NASA finder observations

These 12 JSONL captures retain live WGC2 requests and raw response bodies
from 2026-11-13 (Asia/Shanghai). They cover one case from each of the 11
Geometry Finder families and one additional distance-window postprocessing
case. The corresponding independently captured CSPICE windows and local
query inputs are in `docs/review-2026-11-11/gf-all-families/cases.json`.
`SHA256SUMS.txt` protects the recorded request and response bytes.

Build `wgc_gf` with the applicable pinned numerical profile, then run
`python tools/replay_wgc_finders.py --binary <path> --report <output.json>`.
The replay checks every window endpoint and duration against the retained
CSPICE binary64 values, and compares every returned WGC cell after the
independently probed WGC serialization rule. Missing rows, errors and
differences fail. The report records input and executable hashes.
Server kernel bytes were not independently retrieved; these finite samples
do not establish complete option-level or cross-platform parity.

Eight additional `finder-matrix-*.jsonl` traces contain nine relation and
aberration-correction cases each. `finder-matrix-cases.json` records the
request, native windows, input hashes and differences observed before the
surface-intercept longitude fix. `finder-matrix-after.json` records the
subsequent 72-case run with no displayed or native-bit differences. Its binary
hash identifies the Windows release executable used for that run. The native
reference loaded its listed kernels in order; the backend's SPK, PCK and FK
inputs are listed separately, with no LSK loaded by that backend.

Run `python tools/replay_wgc_finder_matrix.py --binary <path> --glibc-rotation
--report <output.json>` to reproduce the displayed and binary64 comparisons.
It returns failure on any mismatch and records the actual rotation profile.
The three original LT intercept differences remain visible in the before-fix
record; they are not ignored or replaced by a numerical tolerance.

The occultation matrix records 12 correction/type combinations, including
three explicit empty windows. Its replay uses `--occultation` with the matrix
tool; an empty reference requires an empty local result and does not count as
an endpoint comparison. Cassini ray and target FOV traces, native captures
and replay reports are retained separately. Run
`python tools/replay_wgc_ray_stellar.py --binary <path> --report <output.json>`
for the ray's S/XS and invalid LT/CN+S requests; add `--target-fov` for the
three target correction cases. These reports do not establish server kernel
identity or raw server binary64 equality.

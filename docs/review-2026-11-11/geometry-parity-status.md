**Verified non-time WGC family audit — 2026-11-11**

Fresh NASA examples cover all 24 non-time calculation families: 13 geometry families and 11 event-search families. Every returned computational column is checked. The explicit WGC output-serialization comparisons pass; the default raw transported-number comparisons remain FAIL where NASA rounds the underlying results. No tolerance or threshold widening changes acceptance.

The geometry sweep contains 144 local query rows and 71 actual NASA calculations, grouped into 15 reports because state representations and point locations are additional groups. It includes sub-observer, sub-solar, and full typed pointing requests. Its all-family scope is representative sampling, not a proof for every input and every kernel.

| Geometry audit group | Checked query rows | Result |
|---|---:|---|
| State Vector | 1 | Exact WGC output serialization |
| State Representations | 82 | Exact WGC output serialization |
| Angular Separation | 4 | Exact WGC output serialization |
| Phase Angle | 1 | Exact WGC output serialization |
| Angular Size | 3 | Exact WGC output serialization |
| Osculating Elements | 1 | Exact WGC output serialization |
| Planetocentric Solar Longitude | 1 | Exact WGC output serialization |
| Illumination Angles | 1 | Exact WGC output serialization |
| Surface Intercept | 1 | Exact WGC output serialization |
| Tangent Point | 1 | Exact WGC output serialization |
| Sub-observer Point | 4 | Exact WGC output serialization |
| Sub-solar Point | 4 | Exact WGC output serialization |
| Pointing Direction | 7 | Exact WGC output serialization |
| Frame Transformation | 29 | Exact WGC output serialization |
| Locations | 4 | Exact WGC output serialization |

The event-search audit contains 12 requests, covering all 11 official families plus a RANGE/postprocessing case. All 54 native start/stop/duration doubles from 18 nonempty intervals match independent CSPICE N0067 binary64 bits exactly. Their NASA date and duration cells also match the separately declared WGC serialization rules.

| Event-search case | Native intervals | CSPICE bit equality | NASA serialization equality |
|---|---:|---|---|
| distance | 1 | PASS | PASS |
| range_rate | 2 | PASS | PASS |
| coordinate | 1 | PASS | PASS |
| phase | 1 | PASS | PASS |
| separation | 2 | PASS | PASS |
| subpoint | 1 | PASS | PASS |
| illumination | 1 | PASS | PASS |
| intercept | 1 | PASS | PASS |
| occultation | 1 | PASS | PASS |
| range_postprocessing | 5 | PASS | PASS |
| target_fov | 1 | PASS | PASS |
| ray_fov | 1 | PASS | PASS |

NASA dates are compared at the requested TDB-seconds display precision of six decimal places. Numeric output formatting is independently probed: eight fixed decimals for magnitudes from 1e-4 through less than 1e10, otherwise eight decimals in the scientific mantissa, with verified half-up rounding. These rules belong to an explicit output compatibility layer; the internal native values remain unchanged. Default raw equality and native bit equality are separate checks.

The source regressions additionally cover 1,800 explicit target-frame surface cases (17,460 numeric values plus illumination flags), 246 surface cases with a rotating SPK source (2,262 numeric values plus flags), 4,104 mixed-source pointing pairs (24,624 direction components), 200 typed CLI coordinate/FOV outputs, and the prior location/state-coordinate/native geometry batteries. FOV boundary rows retain their one-based instrument order.

Actual KernelSet 1 provenance was obtained from the official GUI RPC result: `pds/wgc/mk/solar_system_v0072.tm`. Its complete 66-file load order is preserved. Full `de442.bsp` is used. All 18 actual FK files were downloaded and checked with native CIDFRM: Earth defaults to `IAU_EARTH`; Moon defaults to `MOON_ME`. `EARTH_FIXED` is an alias for `ITRF93`, which does not replace Earth’s default frame. All 24 fresh NASA Earth clock cases and 24 Moon cases match native ET2LST. Forcing Earth’s default to ITRF93 gives 24 incorrect clock strings.

The rolling Earth BPC was matched by SHA-256 to its immutable original `earth_000101_270205_261110.bpc`; HTTP response date/Last-Modified and both hashes are recorded.

Both FOV event families use real Cassini ISS NAC data. The official KernelSet 5 result identifies `cas_2012_v16.tm`. The selected reconstructed SPK includes Cassini and Enceladus; the covering reconstructed CK, SCLK, FK, IK, pck00010, and the mission Enceladus overlay are pinned. NASA requests explicitly append full DE442. Twelve actual Cassini epochs also match native SCE2C ticks and NAC rotation matrices in all 120 compared components.

A probe that appended generic pck00011 after the mission Enceladus overlay failed on NASA with INSUFFICIENTANGLES (8 angles versus 26 required). That failed transport is retained. The successful requests use the coherent mission PCK order; no model or matrix is substituted.

Primary artifacts:

- `wgc-geometry-final-live-requests.jsonl`, `wgc-geometry-final-live-report.json`, `wgc-geometry-final-live.log`
- `wgc-geometry-final-raw-strict-report.json` preserves the strict transported-value failures.
- `gf-all-families/all-family-output-exact-report.json` includes native bit checks, exact NASA columns, all resource paths/hashes, and an empty missing-family list.
- `gf-all-families/all-family-raw-strict-report.json` preserves the independent raw equality failures.
- `gf-all-families/*.jsonl` contains the genuine request/response transport.
- `wgc-kernel-provenance.json`, `wgc-kernel-fk-provenance.json`, `solar_system_v0072.tm`, `cassini-fov-kernel-provenance.json`, `cas_2012_v16.tm`, and the saved GUI RPC request/response files pin the source data.
- `earth-tangent-clock/` and `tangent-clock/` retain clock probes and native comparisons.

Reproduction from this workspace:

```powershell
python tools/wgc2_geometry_compare.py --quick --wgc-output-exact --replay docs/review-2026-11-11/wgc-geometry-final-live-requests.jsonl --report target/geometry-replay.json
python tools/audit_wgc_gf.py --output-exact --report target/gf-replay.json
python tools/download_cassini_fov_kernels.py --verify-only
cargo test --test gf_live_bridge_exact --test cassini_fov_frames --test surface_target_frames --test surface_rotating_spk --test pointing_mixed_frames --test pointing_cli_exact --bin wgc_geometry
python -m unittest discover -s tools/tests -v
```

Time conversion remains outside this audit, as requested. Subsequent DSK coverage and the repository-wide final gate are tracked separately.

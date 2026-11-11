# TWO_DIRECTIONS evidence — 2026-11-11

`src/separation.rs` implements the ANGULAR_SEPARATION `TWO_DIRECTIONS` specification through the existing typed pointing resolver. Each direction has its own context, allowing different instruments, observers, input frames, corrections and anti-vector flags. The result is one separation per epoch. There is no boundary-vector expansion or pairing operation: the official DirectionSpecification limits FOVBOUND to POINTING_DIRECTION, and a live request confirms that angular separation rejects instrument FOV vectors.

Independent CSPICE N0067/SpiceyPy 8.3.0 fixtures cover 1,980 direction pairs and require identical bits for 17,820 angle, resolved-vector and frame-epoch components. They include POSITION, VELOCITY and VECTOR; all nine state/position corrections and all seven vector corrections; four frames; three epochs; anti-vectors; distinct instrument frames; axes, boresights and instrument-frame vectors; and POINT/SPHERE combinations. Another 204 VSEP cases cover zero vectors, near-parallel/antiparallel vectors and extreme magnitudes. Four successful NASA live-probe inputs have separate CSPICE exact regressions.

Five tests pass with no numerical tolerance:

```powershell
python tools/gen_two_directions_precision_golden.py
cargo test --target-dir target/separation-agent --lib separation:: -- --nocapture
```

`tools/probe_two_directions_wgc.py` preserves six NASA requests and responses in `two-directions-live.json`. NASA rejects SPHERE combined with an anti-vector, and rejects instrument boundary vectors. `two-directions-live-comparison.json` retains the strict decimal comparison with the four successful result rows: one is exact, three differ by 1.103135e-9 to 4.31835e-9 degrees because the returned numbers have eight decimal places. These remain strict transport mismatches; they are not accepted by rounding the local result. The source-level numerical fixtures are exact. This evidence does not establish complete WebGeocalc parity.

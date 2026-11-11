# Validation and independent references

## Requirements

Use Rust 1.75 or newer, rustfmt, Python 3.11 or newer and the pinned SpiceyPy reference environment. Production Rust does not require SpiceyPy or CSPICE.

```sh
python -m pip install -r tools/requirements-reference.txt
python tools/bootstrap_reference.py
python tools/verify_strict.py
```

The bootstrap downloads official files to target/, verifies pinned hashes and never substitutes another kernel. The exact reference profile is Windows x86_64 CSPICE N0067. A different native binary must not silently inherit the Windows bit-equality claim.

The strict gate runs formatting, verifier regressions, Debug and Release Rust tests. It requires the named exact tests, rejects ignored/skipped cases and checks that sources did not change during execution. Its report is target/strict-verification/report.json.

## Additional package checks

```sh
cargo test --locked --all-targets --all-features
cargo +1.75.0 check --locked --all-targets --all-features
cargo build --locked --release --all-features --bins
cargo doc --locked --no-deps --all-features
cargo publish --locked --dry-run
```

MCP protocol/compute regressions are part of the all-feature suite. See [MCP](MCP.md).

## Live NASA audit

The [family audit](review-2026-11-11/geometry-parity-status.md) records all 24 non-time families. Replay genuine captures with explicit output-serialization comparison:

```sh
python tools/wgc2_geometry_compare.py --quick --wgc-output-exact --replay docs/review-2026-11-11/wgc-geometry-final-live-requests.jsonl --report target/geometry-replay.json
python tools/audit_wgc_gf.py --output-exact --report target/gf-replay.json
```

Retain raw-strict failures separately. Replay proves the saved transport; fresh live requests additionally establish current service behavior. Generators and evidence have distinct purposes: generators produce native references, tests compare the implementation, and reports record the exercised scope.

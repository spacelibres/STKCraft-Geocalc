# Numerical contract

## Equality is reference-specific

The target is identical computational results for the same explicit inputs as CSPICE N0067 and WebGeocalc. Binary64 equality is tested against the independent native runtime, not against another call into this Rust implementation. More printed digits do not increase physical accuracy.

Inputs that affect a result include kernel bytes and load order, target/observer/illuminator IDs, frames, ET, aberration mode, shape/surface selection, units, search settings and output representation. A different ephemeris or body-frame association is a different calculation.

## Arithmetic

Source algorithms retain operation grouping, interpolation windows, record selection, branch conditions and iteration schedules. Do not introduce reassociation, approximate trigonometric functions, fitted constants or generic substitute solvers to hide a discrepancy. Position-only and full-state frame paths must remain separate.

GF convergence tolerance controls the search algorithm. It does not authorize a tolerance when comparing the returned windows. Numerical differentiation is used only where it is the actual native algorithm, with the same samples and step rule.

## Results and transport

Compare unrounded native components, flags, counts, ordering and errors. A missing/empty result is accepted only when the independent reference has the same semantic outcome. Nonfinite values cannot establish conformance.

NASA's transported display values are checked through an explicitly selected serialization layer. The raw strict report is retained separately. Display formatting must never modify the internal calculation to force a match.

## Windows runtime profile

The reference profile uses x86_64 Windows, MSVC, a statically linked UCRT,
FMA3-capable hardware with the CRT default enabled, round-to-nearest/ties-to-even,
and the default environment without flushing subnormals. No production code
changes the caller's FMA setting or rounding mode. Source algorithms and every
original expected value are preserved.

Pinning the CSPICE DLL alone does not pin its system math. The independent
[remote captures](reference/RUNTIME_MATH_INVESTIGATION.md) show three VSEP and
eight PROP2B row differences on Windows Server 2022 even with FMA enabled;
the same DLL and inputs have zero such differences on the newer host.
The repository therefore selects `-C target-feature=+crt-static` for MSVC.

Cargo does not inherit a dependency's project configuration, and registry
installation starts configuration discovery from Cargo home. See the official
[Cargo install rules](https://doc.rust-lang.org/stable/cargo/commands/cargo-install.html).
Library consumers must use this in their own `.cargo/config.toml`:

```toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

Registry binary installation must pass that configuration explicitly, as shown
in the README. The prebuilt Windows distribution uses the exact main CI Release
executables, not a later rebuild. CI records their SHA256 and rejects dynamic
CRT imports; packaging rechecks the same bytes.

The additional [native capture](reference/RUNTIME_MATH_CAPTURE.json) records
13,909 input rows and 14,914 output cells from a separate C process using
`/MT /Od /fp:strict`, the official N0067 library and a checksum-pinned SDK UCRT.
The generator receives and returns binary doubles, checks the original 204
VSEP and 288 PROP2B rows unchanged, and refuses to overwrite an existing capture
directory. Compiler diagnostics are retained verbatim as external evidence.

From a provisioned Git checkout with the exact recorded SDK library bytes:

```powershell
python tools/capture_runtime_math.py --output-dir target/new-native-math-capture
cargo test --locked --all-features --test runtime_math_exact -- --nocapture
```

SDK version labels do not identify library bytes: the three observed machines
have different `libucrt.lib` checksums under the same version label. Diagnostics
record the actual linked library inputs. The primitive guard and complete
native corpus must both pass; a matching SDK label or ten sample primitives
alone cannot establish acceptance. Other CPU/runtime/compiler profiles remain
uncertified until independently checked against all required outputs.

## Limits

Every equality claim is limited to the actual reference corpus, environment and data. Build success on a new platform is not evidence of bit equality. The accuracy of an SPK, PCK, CK or DSK data product is independent of arithmetic reproduction. Full option-level certification remains open in [the gap register](STRICT_REMAINING_GAPS.md).

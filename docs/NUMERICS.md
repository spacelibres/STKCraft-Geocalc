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

## Limits

Every equality claim is limited to the actual reference corpus, environment and data. Build success on a new platform is not evidence of bit equality. The accuracy of an SPK, PCK, CK or DSK data product is independent of arithmetic reproduction. Full option-level certification remains open in [the gap register](STRICT_REMAINING_GAPS.md).

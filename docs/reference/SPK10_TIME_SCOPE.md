# Native historical SPK10 time integration

SPK type 10 initializes SGP4 using N0067 TTRANS's scalar JDUTC at each
packet epoch. The geometry evaluator now obtains that value through the
exact registry dependency Time 0.1.30:

```text
TimeContext::new(lsk).et_to_spice_julian_utc(et)?.days()
```

Time remains implemented by the independent Time project. Geometry does not
reimplement its historical offsets or reconstruct JDUTC through integer
Epoch/civil-calendar fields. The explicit path reproduces native's
historical convention; it does not select a physical historical UTC/SOFA
model. The supplied LSK must use IERS-only/TableDriven settings. Other
policies return an explicit model error instead of a substitute result.

## Independent native regression

The unmodified N0067 static library writes the synthetic SPK and evaluates
both SPKEZR and SPKEZP independently of Rust. The same generated SPK bytes,
file load order, original `naif0012.tls`, `de440s.bsp` and `pck00011.tpc`
are used by the Rust comparison. Original inputs, stdout/stderr,
source/generator snapshots, compiler/linker records and hashes are retained
in the [capture evidence](https://github.com/spacelibres/STKCraft-Geocalc/tree/v0.1.11/docs/review-2026-11-12/spk10-historical).

The matrix contains 52 generated segments and 33,696 query groups:

- Thirteen packet-epoch centers spanning 1873 to J2000, including the
  first-LSK-entry boundary.
- Near-Earth and deep-space periods, with one- and three-packet segments.
- Nine epoch offsets, including packet boundaries and blended intervals.
- J2000, B1950, GALACTIC and IAU_EARTH output frames.
- Earth and Moon observers and all nine aberration choices.

Every group checks all six state components, state light time, all three
position-only components and position light time: 370,656 unrounded
binary64 cells. Signed zero is preserved. Both paths execute for every
query; a missing result, nonfinite value or unexpected field count fails.
The original SPK10 fixtures, including 3,045 geometric states and 270
corrected state/position groups, are retained unchanged.

Before repair, 24,336 native-valid groups were rejected with the Time
coverage error. The other groups compared 102,960 cells. The repaired
path matches all 33,696 groups and all 370,656 cells. The original
before/after logs are retained beside the native capture. No input,
expected value, output field or comparison tolerance was changed to
accept a numerical discrepancy.

The C reference uses the pinned N0067 archive/library and static UCRT,
`/Od /fp:strict /MT`, and unchanged default FMA setting. Production remains
pure Rust. See [the numerical profile](../NUMERICS.md) and
[Time's explicit scope](https://github.com/spacelibres/STKCraft-time/blob/v0.1.30/docs/reference/NATIVE_JDUTC.md).

## Limits

This regression uses the recorded original LSK and its constants. It does
not certify every altered LSK/frame/orbit product, malformed SPK or raw
time-pool error. It does not implement every historical civil-text format
or all TTRANS representations. Default civil/scientific time APIs retain
their separate contracts. Complete WebGeocalc parity remains false.

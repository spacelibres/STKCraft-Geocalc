# Time integration gaps exposed by native CK fixtures

Status at initial handoff: external dependency failures were active. See the resolution checkpoint below for the fresh result. The user directed that this evidence be handed to STKCraft-time; no sibling source changes were made by the geometry worker. These are exact comparisons against CSPICE N0067, not tolerance failures.

## Reproduce

From STKCraft-Geocalc with the pinned `naif0012.tls` present:

```powershell
python tools/gen_ck_golden.py
cargo test --target-dir target/gf-agent --test ck_exact native_sclk_ticks_for_ck_frames -- --nocapture
cargo test --target-dir target/gf-agent --test ck_exact native_ck_frames_use_exact_spacecraft_clock_conversion -- --nocapture
cargo test --target-dir target/gf-agent --test ck_exact native_valid_scalar_sclk_kernel_is_accepted -- --nocapture
```

Independent geometry subsets:

```powershell
cargo test --target-dir target/gf-agent --test ck_exact native_ck_encoded_tick_pointing_exact -- --nocapture
cargo test --target-dir target/gf-agent --test ck_exact native_ck_tdb_frame_subset_exact -- --nocapture
```

The encoded-tick engine matches 32,622 native CKGP/CKGPAV calls exactly, including call-history reuse, file/segment priority, all 21 inertial frames, AV flags, interpolation gaps, zero tolerance and directories exceeding 100 interpolation intervals. The separately named TDB frame subset matches its 168 actual inputs. Neither result establishes TDT/SCLK integration parity.

## Evidence, finding and repair path

| Evidence | Observed finding | Source-backed repair path for the time project |
| --- | --- | --- |
| `tests/golden/ck_test_scalar.tsc`; `native_valid_scalar_sclk_kernel_is_accepted` | Native FURNSH accepts scalar assignments such as `SCLK_DATA_TYPE_100 = 1` and `SCLK01_N_FIELDS_100 = 2`. SclkKernel::parse returns SclkParse because parse_list always requires parentheses. | Accept both scalar and parenthesized numeric assignments. Preserve the independent scalar kernel; do not silently rewrite external input in geometry. |
| `tests/golden/ck_sclk_ticks.csv`; `native_sclk_ticks_for_ck_frames` | Nine of 42 SCE2C values differ: two TDB cases and seven TDT cases. For clock -100, ET `1009.7646484375`, native returns `999.9000000000001`; the dependency returns `999.9`. | Port the operation ordering of SC01's SCEC01 entry: compute `rate = 1 / (coefficient_rate / ticks_per_most_significant_count)`, then `ticks = constant + rate * time_difference`. The current multiply-then-divide expression changes the final bit. Also audit SCTE01, which computes seconds-per-tick first before multiplying the tick difference. |
| `tests/golden/ck_sclk_ticks.csv`, clock -300 | For ET `1000.9789315048911`, native SCE2C returns `100.25`; the dependency returns `100.24999997466803`. | Native SCEC01 uses UNITIM(TDB,TDT). The dependency routes through ET→TAI Epoch→TT, introducing large-origin addition/subtraction and extra rounding. Port direct native UNITIM scale conversion and validate both directions. Do not adjust input ET or enlarge CK tolerance. |
| `tests/golden/ck_frames.csv`; `native_ck_frames_use_exact_spacecraft_clock_conversion` | 84 of 420 native frame cases fail through the TDT clock dependency. A tick mismatch can turn an exact CK sample into a zero-tolerance miss or perturb an interpolated matrix. | After the time fixes, rerun the unmodified 42-value and 420-frame tests, plus the time project's independent conformance gate and the complete geometry gate. |

The fixture clock is type 1 with fields `(1000000000, 256)`, nonzero parallel-time origin, two coefficient segments and a changing rate. Clocks -100/-200 use TDB, and -300 uses TDT. `ck_test.tsc` uses parenthesized assignments accepted by both parsers; `ck_test_scalar.tsc` preserves the separately demonstrated native-valid scalar spelling.

Official source pointers in the extracted N0067 toolkit: `sce2c.c` delegates to `sc01.c` SCEC01; that entry calls `unitim.c` for TDT and computes the reciprocal rate before the affine map. SCTE01 in `sc01.c` computes seconds-per-tick before its affine map. `ckfrot.c` and `ckfxfm.c` call SCE2C with zero tick tolerance, which is why these discrepancies cannot be hidden by changing a search tolerance.

## Acceptance

Keep all three dependency tests active. No ignored tests, output exclusions, input nudges or tolerance increases are permitted to turn these results into PASS. CK's tested encoded-tick subset can be reported separately; the complete local gate remains failing until the time project resolves these inputs.

## Resolution checkpoint

On 2026-11-11, STKCraft-time v0.1.17 was rebuilt and the unchanged native fixtures were rerun. All 42 SCE2C values and all scalar-kernel parse cases now pass exactly. No sibling source edits were made by this geometry worker.

The first rerun reduced frame failures from 84 to 10. These remaining cases exposed a GeoCalc FRMCHG/REFCHG error: CK lookup with FOUND=false must stop the from-side chain and still allow the to-side chain to meet its last node. A fixed child relative to an uncovered CK ancestor can therefore be resolved without ancestor pointing. GeoCalc now preserves that source behavior.

Fresh command `cargo test --target-dir target/gf-agent --test ck_exact -- --nocapture`: 6 passed, 0 failed, 0 ignored. This includes all 32,622 encoded-tick pointing calls, all 420 frame calls, all 42 tick values, the 168-case TDB subset, scalar parsing, and DAF-kind rejection. The historical red evidence above remains as the handoff record; it is no longer the current status.

Dependency source SHA256 at this successful checkpoint:

- `src/sclk.rs`: `6babab18859e0b22be9b6bb5ce45115788e0f9929fe817a1f7f042c9466f1df0`

- `src/naif.rs`: `c07950a2b57d8fe67bb3effe1f029452f59c1b5e008eb6056e4b995ef0243074`

- `Cargo.toml`: `ba272643e94efb7cde6bc6390d60f5b31a9712acaa7a1b06ba82f493f3b2f135`

The dependency was subsequently rebuilt as v0.1.18 during SGP4 verification; its independent project owner is responsible for those changes. Fresh source hashes are recorded below and a complete CK retest is part of this checkpoint.

- `src/sclk.rs`: `5dcc56d42280911beae7deb84da54b9ff7a3af36703d7536f0512b34147490eb`

- `src/naif.rs`: `c07950a2b57d8fe67bb3effe1f029452f59c1b5e008eb6056e4b995ef0243074`

- `Cargo.toml`: `e95b8733ce9c39c11c9175f188fac44ca2c379c0e91e2e9baac80882e94f4149`

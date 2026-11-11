---
name: stkcraft-geocalc-mcp
description: Configure, run, test, or use the optional STKCraft-Geocalc stdio MCP server for kernel-backed geometry and event searches. Discover its real tool schemas before calling; time conversion remains in STKCraft-time.
---

# STKCraft Geocalc MCP

This is the optional local geometry server, not the separate STKCraft-time MCP server or NASA's HTTP API. Read docs/MCP.md in the checkout for maintained schemas and examples; use docs/STRICT_REMAINING_GAPS.md for scientific limits.

## Start and discover

Build the server and both calculation backends together:

    cargo build --release --features mcp --bins

Run target/release/stkcraft-geocalc-mcp from the checkout (append .exe on Windows). It uses stdio JSON-RPC. Keep stdout exclusively for protocol messages; diagnostics belong on stderr. Configure an MCP client with an absolute executable path. Relative kernel paths resolve in the server's working directory; prefer absolute paths when the client's working directory is uncertain.

Initialize the connection with the client's supported protocol version and capabilities, send notifications/initialized, then call tools/list. Use the schemas actually returned by that server version. Do not invent arguments, aliases, tool names, or a network endpoint from a WGC example.

The advertised family tools cover geometry calculations and the gf_* event-search families. Route an ordinary six-component Moon/Earth state to state_vector, a direction-only calculation to pointing_direction, and an event interval to the corresponding gf_* tool, such as gf_distance_search. An angular quantity at an epoch is a geometry calculation; searching when it crosses a threshold is a finder calculation.

The calculate_csv tool is an explicit bridge for the advertised calculator modes and CSV queries. Use it only when the family tool does not express the needed option or the user specifically requests the bridge. Its allowlist is not permission to run arbitrary executables, shell commands, environment settings, or files as code.

## Supply a reproducible scientific context

Tools accept a kernels object. Follow its returned schema for SPK, PCK, FK, CK, LSK, SCLK, and optional instrument IK inputs. Arrays preserve loading order; later data can override earlier data. A path to a similarly named subset kernel does not reproduce the requested full ephemeris.

- States and positions need covering SPKs for both bodies and their center chains.
- Body-fixed surfaces require the actual shape/orientation PCK and any frame-association FKs. Dynamic frames need the selected ephemeris attached by the backend.
- Real instrument frames need their CK, FK, SCLK and LSK. Supply spacecraft ID with each SCLK entry and the instrument ID with the IK when required.
- SPK type 10 needs an explicit LSK.
- ET is NAIF seconds past J2000. Convert a UTC or SCLK input through STKCraft-time first, with the supplied clock/leap-second data; do not reinterpret a UTC number as ET.

Check target versus observer, frame, aberration, units, method, shape, direction definition, and target-frame selection before the call. Preserve a user-supplied kernel order. Missing data or unsupported combinations are explicit errors; do not retry with another frame, zero state, identity rotation, or approximate model to manufacture a result.

## Read results without weakening the contract

The adapter returns scientific JSON and retains backend CSV for traceability. Use structuredContent and its columns/rows when present, and retain rawCsv when investigating numeric serialization or mixed boolean/string columns. Preserve all rows, counts, boundary-vector ordering, no-intercept states, and GF start/stop/duration fields.

Distinguish JSON-RPC request/schema failures from tool execution failures marked isError. Check both before presenting an answer. An empty successful GF window means no matching interval under the supplied search settings; it does not mean the calculation failed. Missing backend rows or an ERROR CSV record is not an empty scientific result.

For exact comparisons:

- Use unrounded native CSPICE references with the same kernels and inputs for computational equality.
- Treat NASA transported decimals separately from native f64 values.
- Use the explicit WGC serialization comparison only when that output contract is requested. A serialization match does not prove equality of internal doubles.
- Search step, convergence, confinement, scalar adjustment and window processing must match the reference; they are not permission to widen endpoint comparison thresholds.

## Test or extend the server

Use the static Windows CRT numerical profile documented in docs/NUMERICS.md.
Keep the distributed server and both backends together; the Windows release
contains the exact executables exercised by main CI. Other runtime profiles
require their own independent verification.

Run tools/list against the built executable and inspect the actual schema before modifying dispatch. Keep family tools and calculate_csv on the same validated backend path so errors and numerical behavior agree. Add a protocol regression for any changed request/result mapping, and an independent native numerical case when changing scientific behavior.

Useful checks:

    cargo test --features mcp
    cargo test --release --features mcp
    python tools/verify_strict.py

A realistic read-only forward test is: "Using the supplied DE442 kernel, compute the Moon's six-component state relative to Earth in J2000 at ET=0 with NONE correction, then find intervals where Moon/Earth distance is below 400000 km in ET [0,2592000], using a 3600-second step." Discover schemas, call state_vector and gf_distance_search, and compare every returned component and interval with independently generated CSPICE results. Record the tools, kernel hashes, exact requests and comparison mode. This checks the selected routes; it does not certify every tool or parameter combination.

The [recorded forward test](references/forward-test.json) used the actual stdio server, discovered 25 tools, checked 10 state/derived values and three interval values against native IEEE-754 bits, and verified a missing-kernel error. It was executed by the skill author, not by an independent evaluating agent.

Keep skill/server documentation in English. Follow repository release instructions for development changes; using a local calculation tool does not authorize publishing a release.

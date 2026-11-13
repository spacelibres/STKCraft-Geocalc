# STKCraft GeoCalc

A Next.js and Three.js workspace for the existing STKCraft geometry and time
libraries. The server exposes the geometry backend's discovered calculator and
finder tools, the existing STKCraft-time tools, a checked UTC/NAIF ET bridge,
and a browsable API reference.
The client sends no kernel paths: the server owns the kernel configuration.

## Run locally

From the repository root, obtain the reference kernels using the repository's
existing bootstrap workflow, then build the geometry and time backends:

```sh
cargo build --release --locked --features mcp --bins
cargo build --release --manifest-path web/native/Cargo.toml --locked
cd web
npm ci
npm run dev
```

Open `http://127.0.0.1:3000/` for the workspace and `/api-info` for the
reference. The default configuration uses `target/kernels/de442.bsp`,
`pck00011.tpc`, `mars_iau2000_v1.tpc`, `gm_de431.tpc`,
`moon_pa_de440_200625.bpc`, the two lunar FKs and `naif0012.tls` from the
repository's `target/kernels` directory. It does not include instrument CK,
SCLK or IK data. Configure an appropriate context for instrument calculations.
Missing files, unsupported options and out-of-coverage epochs return errors.

For a production build use `npm run build` followed by `npm run start`.
These scripts bind to loopback. This local service does not implement user
accounts or authentication; deployment outside a trusted local environment
requires an appropriate authenticated deployment layer and resource limits.

## Server configuration

- `STKCRAFT_WEB_MCP`: absolute path of `stkcraft-geocalc-mcp`. Its matching
  `wgc_geometry` and `wgc_gf` binaries must be available beside it.
- `STKCRAFT_WEB_GEOMETRY`: absolute path of the matching `wgc_geometry` executable used for sampled trajectories; it defaults to the repository release build.
- `STKCRAFT_WEB_TIME`: absolute path of the `stkcraft-web-time-bridge` binary.
  By default the release build produced above is used.
- `STKCRAFT_WEB_TIME_MCP`: absolute path of the companion time MCP binary;
  by default it is built alongside the time bridge.
- `STKCRAFT_WEB_LSK`: path to the LSK used by the time bridge and default
  geometry context.
- `STKCRAFT_WEB_KERNELS_JSON`: a server-owned JSON object matching the kernel
  context described by `/api/catalog`. Its SPK, PCK and FK arrays preserve
  load order. Add CK, SCLK, IK and an instrument ID when needed. This setting
  is administrative configuration and is not accepted from client requests.
  Its optional `lsk` must match `STKCRAFT_WEB_LSK`; the server passes that
  same LSK to both time and geometry so they cannot silently diverge.
- `STKCRAFT_WEB_EOP` and `STKCRAFT_WEB_SCLK`: server-owned EOP and SCLK file
  paths for the corresponding time operations. The spacecraft ID is supplied
  as an operation parameter.
- `STKCRAFT_WGC_GLIBC_ROT3=1`: selects the experimental rotation profile only
  if the geometry binaries were built with the `wgc-glibc-rotation` feature.
  The profile and its bounded evidence are described in `docs/NUMERICS.md`.

## API

- `GET /api/catalog`: actual backend-discovered tool names and schemas.
- `POST /api/calculate`: `{ "operation": "state_vector", "parameters":
  { "target": 301, "observer": 399, "epoch_et": 0, "correction": "NONE" } }`.
- `POST /api/scene`: `{ "epoch_et": 0 }` returns Sun and Moon positions relative to Earth plus Sun-relative planetary center or system-barycenter positions. Unavailable barycenters are listed explicitly.
- `POST /api/trajectories`: `{ "epoch_et": 0 }` returns 65 SPICE position samples per body; null samples indicate missing coverage. Line segments between samples are visual interpolation.
- `POST /api/time`: `{ "direction": "utc_to_et", "utc": { "year": 2024,
  "month": 1, "day": 1, "hour": 0, "minute": 0, "second": 0 } }`, or
  `{ "direction": "et_to_utc", "et": 0 }`.
- `POST /api/time/convert`: `{ "operation": "utc_to_et", "parameters":
  { "year": 2024, "month": 1, "day": 1, "hour": 0, "minute": 0,
  "second": 0 } }`, or another discovered STKCraft-time operation.
  Kernel paths are not accepted from the client.

Dates are checked by `stkcraft-time::contract::TimeContext` using the
configured LSK. Its UTC coverage and historical policy apply. The timeline
stores ET as a binary64 value, uses relative slider offsets to avoid large
HTML range values, and sends that same ET to calculations. Finder intervals
are rendered from returned endpoints; the numeric result table retains the
backend values. The three-dimensional scene computes geometric J2000 Sun and Moon positions relative to Earth using the configured local SPICE kernels at the timeline ET. The solar-system view also plots Mercury and Venus centers plus Mars, Jupiter, Saturn, Uranus, Neptune and Pluto system barycenters using the configured SPK, with clearly labeled, exaggerated point markers for visibility. Trajectory lines connect 65 actual SPICE position samples per body over selected time windows centered on the timeline epoch; line segments are visual interpolation and are not calculation results. Missing-coverage samples break the line. Earth–Moon and Sun–Earth views use the same scale for center distances and spherical body radii; a close-up intentionally places the Sun outside the view. Spherical radii are fixed values from NAIF pck00011.tpc, not dynamically read from a custom PCK. They do not represent terrain, body orientation, apparent positions, or trajectories. In the Sun–Earth view Earth and Moon are very small because their sizes are not exaggerated.

The website does not yet provide the full NASA WebGeocalc HTTP protocol or all
time and geometry option combinations. Discovery describes the
actual backend interfaces; NASA display equality, native bit equality and
untested behavior remain separate claims. Consult `docs/WGC_FEATURE_VERIFICATION.md`
and `docs/STRICT_REMAINING_GAPS.md` before claiming complete parity.

## Verification

With the local server running on port 3100, run:

```sh
npm run typecheck
npm run build
npm run test:integration
node --test tests/examples.test.mjs tests/scene.test.mjs tests/result-visualization.test.mjs tests/scene-snapshot.test.mjs
python tests/test_nasa_compare.py
```

The integration checks exercise discovery, a Time-to-geometry calculation,
exact known state and finder results, invalid UTC and request rejection. They
require the configured local kernels and backend binaries. The repository's
native strict gate is separate from these web checks.

## Live NASA comparisons

Geometry and time calculations report a NASA comparison alongside the local result when a verified request mapping is available. Current live mappings cover state vector, angular separation, angular size, phase angle, orbital elements, coordinate/angular-separation/distance/range-rate/phase-angle finders, UTC to ET (Gregorian dates from 1583 onward), and GPS seconds to ET. Other operations report `unsupported`; they are not represented as having been compared. The server does not submit an invalid local calculation.

The default reference is NASA NAIF WGC2 with kernel set 1. Set `STKCRAFT_WEB_WGC_ENDPOINTS` to a JSON array of one to three independently verified NASA HTTPS API endpoints, in failover order. Each object has `name`, `url`, and `kernel_sets` (an array of NASA kernel-set IDs). For example, a single endpoint can be configured as `[{"name":"NASA NAIF WGC2","url":"https://wgc2.jpl.nasa.gov:8443/webgeocalc/api","kernel_sets":[1]}]`. NAIF describes WGC as GUI-only and WGC2 as offering the API; do not assume the former provides a backup API. Transport and server errors may trigger failover; input or application errors do not. The returned comparison identifies the actual endpoint, kernel sets, failed attempts and calculation ID.

Live calls require Python 3 and use its standard HTTPS transport, including system proxy settings. Set `STKCRAFT_WEB_PYTHON` to a trusted Python executable if necessary. Server requests can take up to 25 seconds per configured endpoint. The displayed comparison uses the verified WGC output serialization and exact decimal equality, with no numerical tolerance. It cannot establish binary64 equality or identical local and NASA kernel contents. The panel reports incomplete column mappings, failed requests and unavailable services explicitly. Only run this local server in a trusted environment: it has no authentication or rate limiting.

## Guided examples

The workspace provides selectable, evidence-linked examples for all 13 geometry calculators and 11 event finders, plus four time examples. Loading an example fills its parameters and exact recorded search window; press Calculate for a fresh local result and the available NASA comparison. Archived native event intervals are provided for reference, and depend on the recorded kernel context. Cassini instrument cases explicitly require mission SPK, FK, CK, SCLK and IK kernels. The NASA live comparison is not implemented for every operation. Successful state and surface-point calculations display their returned components in a separate normalized three-dimensional vector view. Angular results use schematic rays with the computed angle. Finder results display the returned intervals on the timeline; moving the timeline within the search window retains the result. During playback, the scene retains its last completed position and trajectory frame until a matching new frame is ready. If the displayed scene lags the timeline, its ET is shown explicitly. Other operations display their returned values until their spatial illustrations are implemented. Diagram scales, endpoint markers and orientation do not change numerical results. Changing inputs invalidates the corresponding result display.

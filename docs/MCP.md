# MCP Server

The optional `stkcraft-geocalc-mcp` server exposes 24 geometry and Geometry Finder families plus a CSV adapter. Calculations reuse the existing `wgc_geometry` and `wgc_gf` executables. The adapter does not reimplement numerical algorithms or call CSPICE at runtime.

## Build and install

```powershell
cargo build --release --features mcp --bins
cargo install --path . --features mcp --bins
```

Keep all three executables in the same directory. The server launches only the fixed sibling backend filenames, without a shell or PATH lookup. Requests cannot select arbitrary programs, command-line arguments or environment variables. Kernels are user-provided local files; they are not bundled.

Example client configuration:

```json
{"mcpServers":{"stkcraft-geocalc":{"command":"C:/tools/stkcraft-geocalc-mcp.exe","args":[]}}}
```

Use the installed executable path for your platform. Calls are synchronous. Closing stdin terminates the server after the active calculation. HTTP, streaming progress, cancellation, background services and network kernel downloads are not implemented.

## Protocol and resources

The transport is newline-delimited UTF-8 JSON-RPC 2.0. Stdout contains protocol responses only; diagnostics go to stderr. Each input message is limited to 1 MiB. Notifications receive no response. Malformed JSON, invalid IDs and unknown methods produce errors without terminating the process.

Legacy protocol versions `2025-11-25`, `2025-06-18` and `2024-11-05` use `initialize`, followed by `notifications/initialized`. Unsupported legacy version requests receive version `2025-11-25`, allowing the client to decide whether to continue. Operations before initialization are rejected; ping is allowed.

For revision `2026-07-28`, include `params._meta` with these required fields on every request:

```json
{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"example","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}
```

Modern requests need no initialization session. `server/discover` reports the supported versions, identity and capabilities. Unsupported per-request versions receive `-32022`; successful modern results include `resultType: "complete"`.

Implemented methods are initialize, server/discover, ping, tools/list, tools/call, resources/list, resources/templates/list and resources/read. Tool discovery is deterministic and unpaginated; cursors are rejected. `geocalc://manual` is the read-only resource containing this document.

References: official [stdio transport](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports), [legacy lifecycle](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle), [modern versioning](https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning) and [discovery](https://modelcontextprotocol.io/specification/2026-07-28/server/discover). The published [STKCraft Time MCP source](https://github.com/spacelibres/STKCraft-time/blob/main/src/mcp.rs) supplied the project pattern; this adapter has its own validation and dispatch.

## Kernel context

Every calculation requires a `kernels` object. Paths are absolute or relative to the server working directory; arrays preserve load order. Ambient STKCRAFT_SPK/LSK/CK/SCLK variables are cleared in child processes and replaced only by this explicit context.

| Key | Type | Meaning |
| --- | --- | --- |
| spk | non-empty string array, required | SPK files in load order |
| pck | string array | Text and supported binary PCK files |
| fk | string array | Frame kernels |
| ck | string array | CK files |
| lsk | string | Explicit leap-second kernel |
| sclk | array of `{spacecraft_id, path}` | SCLK files; requires explicit LSK |
| ik | string | Instrument kernel |
| instrument_id | integer | Required with IK and rejected without IK |

Unknown context keys are rejected. Paths cannot contain comma, pipe, NUL or newline, which are separators in the existing backend contract. FOV tools require a matching IK and instrument ID; CK evaluation needs the corresponding SCLK and LSK. Missing files or insufficient coverage produce tool execution errors.

## Units and results

Epochs and confinement endpoints are ET seconds past J2000; distances are km, velocities km/s and GM km^3/s^2. GF angular references and adjustments are degrees, distance references km, and range-rate references km/s. The GF step is positive and finite. Returned interval duration units follow `duration_units`. Direction components are passed unchanged to the backend.

Successful results contain text JSON plus this `structuredContent` envelope:

```json
{"calculator":"state","backend":"wgc_geometry","columns":["target","observer","epoch_et","correction","x_km"],"rows":[[301.0,399.0,0.0,"NONE",-291608.3847334454]],"rawCsv":"complete original backend output"}
```

The abbreviated example illustrates the envelope; actual results include every native output column. Numbers round-trip as the exact f64 values printed by the backend. The optional serde_json dependency enables float_roundtrip for input parsing. `rawCsv` preserves the complete original output, including final newlines. The adapter performs no unit conversion or decimal rounding. Boolean cells remain booleans, empty cells become null, and text remains text. Header-producing modes retain their native column IDs. Variable-width legacy modes use field_1, field_2, etc. when no matching fixed header exists; consult their raw contract below. Non-finite input or output numbers are rejected.

Malformed arguments and unknown tools return JSON-RPC -32602. A valid request whose backend cannot calculate returns `isError: true` with a diagnostic in text content. Check isError before consuming scientific data. Backend diagnostics cannot escape onto the MCP stdout protocol stream.

## Examples

Initialize a legacy client and discover the actual schemas:

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"example","version":"1"}}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/list"}
```

Moon state relative to Earth:

```json
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"state_vector","arguments":{"kernels":{"spk":["C:/kernels/de442s.bsp"]},"target":301,"observer":399,"epoch_et":0.0,"correction":"CN+S"}}}
```

Native serial RANGE search:

```json
{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"gf_distance_search","arguments":{"kernels":{"spk":["C:/kernels/de442s.bsp"]},"target":301,"observer":399,"confinement":[{"start":0.0,"end":4000000.0}],"step_seconds":3600.0,"relation":"RANGE","reference":380000.0,"upper":400000.0}}}
```

GF rows contain start_et, end_et and duration. RANGE and postprocessing reuse the existing native finder and window paths.

## Typed tool arguments

Every tool also requires kernels. Tables below are generated from the actual tools/list schemas. Required means omission is an error; other values are defaults. The live schema is authoritative. target_frame defaults to IAU_<target>. Confinement is an array of `{start, end}` ET objects. Extra argument keys are rejected.

### `state_vector`

| Argument | JSON type | Required or default |
| --- | --- | --- |
| `correction` | string | `"NONE"` |
| `epoch_et` | number | `required` |
| `observer` | integer | `required` |
| `target` | integer | `required` |

### `angular_separation`

| Argument | JSON type | Required or default |
| --- | --- | --- |
| `correction` | string | `"NONE"` |
| `epoch_et` | number | `required` |
| `observer` | integer | `required` |
| `shape1` | string | `"POINT"` |
| `shape2` | string | `"POINT"` |
| `target1` | integer | `required` |
| `target2` | integer | `required` |

### `phase_angle`

| Argument | JSON type | Required or default |
| --- | --- | --- |
| `correction` | string | `"NONE"` |
| `epoch_et` | number | `required` |
| `illuminator` | integer | `10` |
| `observer` | integer | `required` |
| `target` | integer | `required` |

### `angular_size`

| Argument | JSON type | Required or default |
| --- | --- | --- |
| `correction` | string | `"NONE"` |
| `epoch_et` | number | `required` |
| `method` | string | `"MAX_RADIUS"` |
| `observer` | integer | `required` |
| `target` | integer | `required` |

### `osculating_elements`

| Argument | JSON type | Required or default |
| --- | --- | --- |
| `center` | integer | `required` |
| `correction` | string | `"NONE"` |
| `epoch_et` | number | `required` |
| `gm_source` | string | `"CENTER"` |
| `reference_frame` | string | `"J2000"` |
| `target` | integer | `required` |

### `sub_solar_point`

| Argument | JSON type | Required or default |
| --- | --- | --- |
| `correction` | string | `"NONE"` |
| `epoch_et` | number | `required` |
| `method` | string | `"NEAR_POINT/ELLIPSOID"` |
| `observer` | integer | `required` |
| `target` | integer | `required` |
| `target_frame` | string | `"IAU_<target>"` |

### `sub_observer_point`

| Argument | JSON type | Required or default |
| --- | --- | --- |
| `correction` | string | `"NONE"` |
| `epoch_et` | number | `required` |
| `method` | string | `"NEAR_POINT/ELLIPSOID"` |
| `observer` | integer | `required` |
| `target` | integer | `required` |
| `target_frame` | string | `"IAU_<target>"` |

### `planetocentric_solar_longitude`

| Argument | JSON type | Required or default |
| --- | --- | --- |
| `correction` | string | `"NONE"` |
| `epoch_et` | number | `required` |
| `target` | integer | `required` |

### `illumination_angles`

| Argument | JSON type | Required or default |
| --- | --- | --- |
| `altitude_km` | number | `0.0` |
| `correction` | string | `"NONE"` |
| `epoch_et` | number | `required` |
| `illuminator` | integer | `10` |
| `latitude_deg` | number | `required` |
| `longitude_deg` | number | `required` |
| `observer` | integer | `required` |
| `target` | integer | `required` |
| `target_frame` | string | `"IAU_<target>"` |

### `surface_intercept_point`

| Argument | JSON type | Required or default |
| --- | --- | --- |
| `correction` | string | `"NONE"` |
| `direction_frame` | string | `"J2000"` |
| `epoch_et` | number | `required` |
| `observer` | integer | `required` |
| `target` | integer | `required` |
| `target_frame` | string | `"IAU_<target>"` |
| `x` | number | `required` |
| `y` | number | `required` |
| `z` | number | `required` |

### `tangent_point`

| Argument | JSON type | Required or default |
| --- | --- | --- |
| `correction` | string | `"NONE"` |
| `correction_locus` | string | `"TANGENT_POINT"` |
| `epoch_et` | number | `required` |
| `observer` | integer | `required` |
| `target` | integer | `required` |
| `target_frame` | string | `"IAU_<target>"` |
| `x` | number | `0.0` |
| `y` | number | `0.0` |
| `z` | number | `0.0` |

### `pointing_direction`

| Argument | JSON type | Required or default |
| --- | --- | --- |
| `anti` | boolean | `false` |
| `axis` | string | `"X"` |
| `azimuth_counterclockwise` | boolean | `true` |
| `correction` | string | `"NONE"` |
| `direction_frame` | string | `"J2000"` |
| `direction_type` | string | `"POSITION"` |
| `elevation_positive_z` | boolean | `true` |
| `epoch_et` | number | `required` |
| `magnitude` | string | `"UNIT"` |
| `observer` | integer | `required` |
| `output_frame` | string | `"J2000"` |
| `representation` | string | `"RECTANGULAR"` |
| `target` | integer | `0` |
| `vector_type` | string | `"VECTOR"` |
| `x` | number | `0.0` |
| `y` | number | `0.0` |
| `z` | number | `0.0` |

### `frame_transformation`

| Argument | JSON type | Required or default |
| --- | --- | --- |
| `angular_units` | string | `"rad"` |
| `angular_velocity` | string | `"NOT_INCLUDED"` |
| `angular_velocity_axes` | string | `"XYZ"` |
| `angular_velocity_units` | string | `"rad/s"` |
| `axes` | string | `"ZYZ"` |
| `correction` | string | `"NONE"` |
| `epoch_et` | number | `required` |
| `frame1` | string | `required` |
| `frame2` | string | `required` |
| `orientation` | string | `"MATRIX_ALL_ONE_ROW"` |
| `time_location` | string | `"FRAME1"` |

### `gf_coordinate_search`

| Argument | JSON type | Required or default |
| --- | --- | --- |
| `adjustment` | number | `0.0` |
| `angle` | string | `"PHASE"` |
| `complement` | boolean | `false` |
| `confinement` | array | `required` |
| `coordinate` | string | `"X"` |
| `coordinate_system` | string | `"RECTANGULAR"` |
| `correction` | string | `"NONE"` |
| `duration_units` | string | `"SECONDS"` |
| `filter` | boolean | `false` |
| `filter_threshold` | number | `0.0` |
| `filter_units` | string | `"SECONDS"` |
| `illuminator` | integer | `10` |
| `interval_adjustment` | string | `"NONE"` |
| `interval_amount` | number | `0.0` |
| `interval_units` | string | `"SECONDS"` |
| `method` | string | `"NEAR_POINT/ELLIPSOID"` |
| `observer` | integer | `required` |
| `occultation_type` | string | `"ANY"` |
| `reference` | number | `0.0` |
| `reference_frame` | string | `"J2000"` |
| `relation` | string | `">"` |
| `second` | integer | `10` |
| `shape1` | string | `"POINT"` |
| `shape2` | string | `"POINT"` |
| `step_seconds` | number | `required` |
| `target` | integer | `required` |
| `target_frame` | string | `"IAU_<target>"` |
| `upper` | number | `0.0` |
| `x` | number | `0.0` |
| `y` | number | `0.0` |
| `z` | number | `0.0` |

### `gf_angular_separation_search`

| Argument | JSON type | Required or default |
| --- | --- | --- |
| `adjustment` | number | `0.0` |
| `angle` | string | `"PHASE"` |
| `complement` | boolean | `false` |
| `confinement` | array | `required` |
| `coordinate` | string | `"X"` |
| `coordinate_system` | string | `"RECTANGULAR"` |
| `correction` | string | `"NONE"` |
| `duration_units` | string | `"SECONDS"` |
| `filter` | boolean | `false` |
| `filter_threshold` | number | `0.0` |
| `filter_units` | string | `"SECONDS"` |
| `illuminator` | integer | `10` |
| `interval_adjustment` | string | `"NONE"` |
| `interval_amount` | number | `0.0` |
| `interval_units` | string | `"SECONDS"` |
| `method` | string | `"NEAR_POINT/ELLIPSOID"` |
| `observer` | integer | `required` |
| `occultation_type` | string | `"ANY"` |
| `reference` | number | `0.0` |
| `reference_frame` | string | `"J2000"` |
| `relation` | string | `">"` |
| `second` | integer | `10` |
| `shape1` | string | `"POINT"` |
| `shape2` | string | `"POINT"` |
| `step_seconds` | number | `required` |
| `target` | integer | `required` |
| `target_frame` | string | `"IAU_<target>"` |
| `upper` | number | `0.0` |
| `x` | number | `0.0` |
| `y` | number | `0.0` |
| `z` | number | `0.0` |

### `gf_distance_search`

| Argument | JSON type | Required or default |
| --- | --- | --- |
| `adjustment` | number | `0.0` |
| `angle` | string | `"PHASE"` |
| `complement` | boolean | `false` |
| `confinement` | array | `required` |
| `coordinate` | string | `"X"` |
| `coordinate_system` | string | `"RECTANGULAR"` |
| `correction` | string | `"NONE"` |
| `duration_units` | string | `"SECONDS"` |
| `filter` | boolean | `false` |
| `filter_threshold` | number | `0.0` |
| `filter_units` | string | `"SECONDS"` |
| `illuminator` | integer | `10` |
| `interval_adjustment` | string | `"NONE"` |
| `interval_amount` | number | `0.0` |
| `interval_units` | string | `"SECONDS"` |
| `method` | string | `"NEAR_POINT/ELLIPSOID"` |
| `observer` | integer | `required` |
| `occultation_type` | string | `"ANY"` |
| `reference` | number | `0.0` |
| `reference_frame` | string | `"J2000"` |
| `relation` | string | `">"` |
| `second` | integer | `10` |
| `shape1` | string | `"POINT"` |
| `shape2` | string | `"POINT"` |
| `step_seconds` | number | `required` |
| `target` | integer | `required` |
| `target_frame` | string | `"IAU_<target>"` |
| `upper` | number | `0.0` |
| `x` | number | `0.0` |
| `y` | number | `0.0` |
| `z` | number | `0.0` |

### `gf_sub_point_search`

| Argument | JSON type | Required or default |
| --- | --- | --- |
| `adjustment` | number | `0.0` |
| `angle` | string | `"PHASE"` |
| `complement` | boolean | `false` |
| `confinement` | array | `required` |
| `coordinate` | string | `"X"` |
| `coordinate_system` | string | `"RECTANGULAR"` |
| `correction` | string | `"NONE"` |
| `duration_units` | string | `"SECONDS"` |
| `filter` | boolean | `false` |
| `filter_threshold` | number | `0.0` |
| `filter_units` | string | `"SECONDS"` |
| `illuminator` | integer | `10` |
| `interval_adjustment` | string | `"NONE"` |
| `interval_amount` | number | `0.0` |
| `interval_units` | string | `"SECONDS"` |
| `method` | string | `"NEAR_POINT/ELLIPSOID"` |
| `observer` | integer | `required` |
| `occultation_type` | string | `"ANY"` |
| `reference` | number | `0.0` |
| `reference_frame` | string | `"J2000"` |
| `relation` | string | `">"` |
| `second` | integer | `10` |
| `shape1` | string | `"POINT"` |
| `shape2` | string | `"POINT"` |
| `step_seconds` | number | `required` |
| `target` | integer | `required` |
| `target_frame` | string | `"IAU_<target>"` |
| `upper` | number | `0.0` |
| `x` | number | `0.0` |
| `y` | number | `0.0` |
| `z` | number | `0.0` |

### `gf_occultation_search`

| Argument | JSON type | Required or default |
| --- | --- | --- |
| `adjustment` | number | `0.0` |
| `angle` | string | `"PHASE"` |
| `complement` | boolean | `false` |
| `confinement` | array | `required` |
| `coordinate` | string | `"X"` |
| `coordinate_system` | string | `"RECTANGULAR"` |
| `correction` | string | `"NONE"` |
| `duration_units` | string | `"SECONDS"` |
| `filter` | boolean | `false` |
| `filter_threshold` | number | `0.0` |
| `filter_units` | string | `"SECONDS"` |
| `illuminator` | integer | `10` |
| `interval_adjustment` | string | `"NONE"` |
| `interval_amount` | number | `0.0` |
| `interval_units` | string | `"SECONDS"` |
| `method` | string | `"NEAR_POINT/ELLIPSOID"` |
| `observer` | integer | `required` |
| `occultation_type` | string | `"ANY"` |
| `reference` | number | `0.0` |
| `reference_frame` | string | `"J2000"` |
| `relation` | string | `">"` |
| `second` | integer | `10` |
| `shape1` | string | `"ELLIPSOID"` |
| `shape2` | string | `"POINT"` |
| `step_seconds` | number | `required` |
| `target` | integer | `required` |
| `target_frame` | string | `"IAU_<target>"` |
| `upper` | number | `0.0` |
| `x` | number | `0.0` |
| `y` | number | `0.0` |
| `z` | number | `0.0` |

### `gf_surface_intercept_point_search`

| Argument | JSON type | Required or default |
| --- | --- | --- |
| `adjustment` | number | `0.0` |
| `angle` | string | `"PHASE"` |
| `complement` | boolean | `false` |
| `confinement` | array | `required` |
| `coordinate` | string | `"X"` |
| `coordinate_system` | string | `"RECTANGULAR"` |
| `correction` | string | `"NONE"` |
| `duration_units` | string | `"SECONDS"` |
| `filter` | boolean | `false` |
| `filter_threshold` | number | `0.0` |
| `filter_units` | string | `"SECONDS"` |
| `illuminator` | integer | `10` |
| `interval_adjustment` | string | `"NONE"` |
| `interval_amount` | number | `0.0` |
| `interval_units` | string | `"SECONDS"` |
| `method` | string | `"NEAR_POINT/ELLIPSOID"` |
| `observer` | integer | `required` |
| `occultation_type` | string | `"ANY"` |
| `reference` | number | `0.0` |
| `reference_frame` | string | `"J2000"` |
| `relation` | string | `">"` |
| `second` | integer | `10` |
| `shape1` | string | `"POINT"` |
| `shape2` | string | `"POINT"` |
| `step_seconds` | number | `required` |
| `target` | integer | `required` |
| `target_frame` | string | `"IAU_<target>"` |
| `upper` | number | `0.0` |
| `x` | number | `required` |
| `y` | number | `required` |
| `z` | number | `required` |

### `gf_target_in_instrument_fov_search`

| Argument | JSON type | Required or default |
| --- | --- | --- |
| `adjustment` | number | `0.0` |
| `angle` | string | `"PHASE"` |
| `complement` | boolean | `false` |
| `confinement` | array | `required` |
| `coordinate` | string | `"X"` |
| `coordinate_system` | string | `"RECTANGULAR"` |
| `correction` | string | `"NONE"` |
| `duration_units` | string | `"SECONDS"` |
| `filter` | boolean | `false` |
| `filter_threshold` | number | `0.0` |
| `filter_units` | string | `"SECONDS"` |
| `illuminator` | integer | `10` |
| `interval_adjustment` | string | `"NONE"` |
| `interval_amount` | number | `0.0` |
| `interval_units` | string | `"SECONDS"` |
| `method` | string | `"NEAR_POINT/ELLIPSOID"` |
| `observer` | integer | `required` |
| `occultation_type` | string | `"ANY"` |
| `reference` | number | `0.0` |
| `reference_frame` | string | `"J2000"` |
| `relation` | string | `">"` |
| `second` | integer | `10` |
| `shape1` | string | `"POINT"` |
| `shape2` | string | `"POINT"` |
| `step_seconds` | number | `required` |
| `target` | integer | `required` |
| `target_frame` | string | `"IAU_<target>"` |
| `upper` | number | `0.0` |
| `x` | number | `0.0` |
| `y` | number | `0.0` |
| `z` | number | `0.0` |

### `gf_ray_in_fov_search`

| Argument | JSON type | Required or default |
| --- | --- | --- |
| `adjustment` | number | `0.0` |
| `angle` | string | `"PHASE"` |
| `complement` | boolean | `false` |
| `confinement` | array | `required` |
| `coordinate` | string | `"X"` |
| `coordinate_system` | string | `"RECTANGULAR"` |
| `correction` | string | `"NONE"` |
| `duration_units` | string | `"SECONDS"` |
| `filter` | boolean | `false` |
| `filter_threshold` | number | `0.0` |
| `filter_units` | string | `"SECONDS"` |
| `illuminator` | integer | `10` |
| `interval_adjustment` | string | `"NONE"` |
| `interval_amount` | number | `0.0` |
| `interval_units` | string | `"SECONDS"` |
| `method` | string | `"NEAR_POINT/ELLIPSOID"` |
| `observer` | integer | `required` |
| `occultation_type` | string | `"ANY"` |
| `reference` | number | `0.0` |
| `reference_frame` | string | `"J2000"` |
| `relation` | string | `">"` |
| `second` | integer | `10` |
| `shape1` | string | `"POINT"` |
| `shape2` | string | `"POINT"` |
| `step_seconds` | number | `required` |
| `target` | integer | `0` |
| `target_frame` | string | `"IAU_<target>"` |
| `upper` | number | `0.0` |
| `x` | number | `required` |
| `y` | number | `required` |
| `z` | number | `required` |

### `gf_range_rate_search`

| Argument | JSON type | Required or default |
| --- | --- | --- |
| `adjustment` | number | `0.0` |
| `angle` | string | `"PHASE"` |
| `complement` | boolean | `false` |
| `confinement` | array | `required` |
| `coordinate` | string | `"X"` |
| `coordinate_system` | string | `"RECTANGULAR"` |
| `correction` | string | `"NONE"` |
| `duration_units` | string | `"SECONDS"` |
| `filter` | boolean | `false` |
| `filter_threshold` | number | `0.0` |
| `filter_units` | string | `"SECONDS"` |
| `illuminator` | integer | `10` |
| `interval_adjustment` | string | `"NONE"` |
| `interval_amount` | number | `0.0` |
| `interval_units` | string | `"SECONDS"` |
| `method` | string | `"NEAR_POINT/ELLIPSOID"` |
| `observer` | integer | `required` |
| `occultation_type` | string | `"ANY"` |
| `reference` | number | `0.0` |
| `reference_frame` | string | `"J2000"` |
| `relation` | string | `">"` |
| `second` | integer | `10` |
| `shape1` | string | `"POINT"` |
| `shape2` | string | `"POINT"` |
| `step_seconds` | number | `required` |
| `target` | integer | `required` |
| `target_frame` | string | `"IAU_<target>"` |
| `upper` | number | `0.0` |
| `x` | number | `0.0` |
| `y` | number | `0.0` |
| `z` | number | `0.0` |

### `gf_phase_angle_search`

| Argument | JSON type | Required or default |
| --- | --- | --- |
| `adjustment` | number | `0.0` |
| `angle` | string | `"PHASE"` |
| `complement` | boolean | `false` |
| `confinement` | array | `required` |
| `coordinate` | string | `"X"` |
| `coordinate_system` | string | `"RECTANGULAR"` |
| `correction` | string | `"NONE"` |
| `duration_units` | string | `"SECONDS"` |
| `filter` | boolean | `false` |
| `filter_threshold` | number | `0.0` |
| `filter_units` | string | `"SECONDS"` |
| `illuminator` | integer | `10` |
| `interval_adjustment` | string | `"NONE"` |
| `interval_amount` | number | `0.0` |
| `interval_units` | string | `"SECONDS"` |
| `method` | string | `"NEAR_POINT/ELLIPSOID"` |
| `observer` | integer | `required` |
| `occultation_type` | string | `"ANY"` |
| `reference` | number | `0.0` |
| `reference_frame` | string | `"J2000"` |
| `relation` | string | `">"` |
| `second` | integer | `10` |
| `shape1` | string | `"POINT"` |
| `shape2` | string | `"POINT"` |
| `step_seconds` | number | `required` |
| `target` | integer | `required` |
| `target_frame` | string | `"IAU_<target>"` |
| `upper` | number | `0.0` |
| `x` | number | `0.0` |
| `y` | number | `0.0` |
| `z` | number | `0.0` |

### `gf_illumination_angles_search`

| Argument | JSON type | Required or default |
| --- | --- | --- |
| `adjustment` | number | `0.0` |
| `angle` | string | `"PHASE"` |
| `complement` | boolean | `false` |
| `confinement` | array | `required` |
| `coordinate` | string | `"X"` |
| `coordinate_system` | string | `"RECTANGULAR"` |
| `correction` | string | `"NONE"` |
| `duration_units` | string | `"SECONDS"` |
| `filter` | boolean | `false` |
| `filter_threshold` | number | `0.0` |
| `filter_units` | string | `"SECONDS"` |
| `illuminator` | integer | `10` |
| `interval_adjustment` | string | `"NONE"` |
| `interval_amount` | number | `0.0` |
| `interval_units` | string | `"SECONDS"` |
| `method` | string | `"NEAR_POINT/ELLIPSOID"` |
| `observer` | integer | `required` |
| `occultation_type` | string | `"ANY"` |
| `reference` | number | `0.0` |
| `reference_frame` | string | `"J2000"` |
| `relation` | string | `">"` |
| `second` | integer | `10` |
| `shape1` | string | `"POINT"` |
| `shape2` | string | `"POINT"` |
| `step_seconds` | number | `required` |
| `target` | integer | `required` |
| `target_frame` | string | `"IAU_<target>"` |
| `upper` | number | `0.0` |
| `x` | number | `required` |
| `y` | number | `required` |
| `z` | number | `required` |

## CSV adapter

calculate_csv takes calculator, queries and kernels. queries must contain exactly one CSV query per call; quoted CSV and embedded newlines are not supported. One query may produce several native rows, such as Euler alternatives, FOV boundary vectors or GF intervals. Send separate calls for separate queries.

```json
{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"calculate_csv","arguments":{"kernels":{"spk":["C:/kernels/de442s.bsp"]},"calculator":"state","queries":["301,399,0.0,CN+S"]}}}
```

| Calculator | Ordered CSV input fields |
| --- | --- |
| state | target, observer, et, correction |
| separation | observer, target1, target2, et, correction, optional shape1, shape2 |
| phase | target, illuminator, observer, et, correction |
| size | observer, target, et, correction, optional MAX_RADIUS/MIN_RADIUS/LIMB_EXTENTS |
| state_rep | target, observer, et, correction, frame_code, representation, azccw, elplsz |
| elements | orbiting, center, et, correction, optional GM source, inertial frame |
| subpoint | solar/observer, target, observer, et, correction, optional method, target_frame |
| longitude | target, et, correction |
| illum | target, observer, longitude_deg, latitude_deg, altitude_km, et, correction, optional illuminator, target_frame |
| intercept_vec | target, observer, et, correction, direction_frame, x, y, z, optional target_frame |
| intercept | target, observer, et, correction, optional target_frame; center ray |
| tangent | target, observer, et, correction, x, y, z, optional correction_locus, target_frame; zero direction selects center |
| pointing | target, observer, et, correction, optional output_frame |
| pointing_full | direction_type, target, observer, et, correction, input_frame, output_frame, magnitude, representation, anti, vector_type, x, y, z, axis, azccw, elplsz |
| frame | from, to, et; output includes row-major rotation components |
| frame_rep | frame1, frame2, et, orientation, axes, angular_velocity, angular_velocity_axes, angular_units, optional correction, time_location, angular_velocity_units |
| location | target endpoint 11 fields, observer endpoint 11 fields, output_frame, frame_type, locus, correction, representation, azccw (0/1), elplsz (0/1), et |

state_rep frame_code is 0 for J2000, -1 for ECLIPJ2000, or a positive NAIF body ID. The GM source is CENTER, CENTER_ORBITER, or a numeric GM string. Each location endpoint's 11 fields are type, objectID, centerID, frame, x, y, z, vx, vy, vz, reference_epoch. Use OBJECT for a body or a point kind for fixed/constant-velocity coordinates. TOPOCENTRIC_AT_POINT requires a point observer; otherwise use a SPICE frame.

The 11 uppercase GF family names corresponding to the typed gf_* tools are also valid calculators. They all take exactly 33 fields:

```text
id,family,target,observer,second,illuminator,frame,target_frame,method,
system,coordinate,angle,relation,lower,upper,correction,confinement,step,
adjustment,complement,window_adjustment,amount,amount_units,filter,
threshold,threshold_units,duration_units,x,y,z,shape1,shape2,occultation_type
```

Confinement is packed as start:end;start:end in ET seconds. Boolean fields are true or false. The family field selects the native finder. BEGIN/ROW/END output is retained in rawCsv and returned as structured interval rows.

## Scope and verification

The tools expose existing CLI calculation paths and supported options, not every WebGeocalc option. The typed state tool returns a J2000 object-object state; state_rep/location expose existing alternate-state paths through calculate_csv. Typed angular separation supports object centers and POINT/SPHERE models; the separate Rust TWO_DIRECTIONS API is not newly wrapped. Current CLI surface modes use ellipsoids, so the new DSK geometry remains available through Rust APIs. Surface-name resolution, DSK endpoint facades and other pending work remain in [strict remaining gaps](STRICT_REMAINING_GAPS.md).

Time conversion remains in the separate [STKCraft Time](https://github.com/spacelibres/STKCraft-time) package and MCP server. This adapter does not duplicate time algorithms or advertise TIME_CONVERSION.

tests/mcp_stdio.rs launches the real server and sibling calculators. It covers initialization, modern metadata, discovery, notification silence, error recovery, finite argument validation, resources and missing-kernel isolation. The Moon-state test compares the complete raw backend output and scientific f64 bits. The Moon-Earth RANGE test matches three independent CSPICE interval pairs exactly. These are adapter checks and do not expand the underlying numerical coverage claim.

```powershell
cargo test --features mcp --test mcp_stdio
cargo clippy --all-targets --all-features -- -D warnings
```

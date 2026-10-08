# uma-sim MCP stdio bridge

Start the Rust API with `uma-sim serve`, then connect an MCP client to:

```sh
node packages/uma-sim-mcp/mcp-stdio.js
```

Set `UMA_SIM_API` to use an API address other than `http://127.0.0.1:8765`.
The bridge uses Node built-ins and needs no package installation.

## Failure responses

The bridge checks HTTP status before treating an API response as data:

| Failure | MCP response |
| --- | --- |
| A tool's REST request returns a non-success HTTP status | Tool result with `isError: true`; text includes the HTTP status and backend response body |
| A resource's REST request returns a non-success HTTP status | JSON-RPC error `-32603`; no resource contents are returned |
| Connection failure or invalid JSON in a successful HTTP response | JSON-RPC error `-32603`, preserving the existing bridge behavior |
| Invalid JSON-RPC request frame, including a null, fractional, or nonfinite ID | JSON-RPC error `-32600` with `id: null`, before a REST request is sent |
| Unknown tool/resource or malformed parameters | JSON-RPC error `-32602`, before a REST request is sent |
| Invalid JSON text | JSON-RPC error `-32700` |

For example, calling `sim_state` before a career exists returns a tool error
containing `HTTP 404` and the API's `no active run` explanation. A client can
then call `sim_start` and retry. A failed `sim_fast_forward` similarly retains
the backend's failure explanation instead of looking like a completed career.

Tool arguments must be an object. Values for advertised properties must match
their string or number types. An omitted arguments object still uses each
tool's existing defaults, and extra properties remain accepted. Successful
tool payloads and resource contents keep their existing formats. Request IDs
must be strings or integers, including zero. Only an omitted ID is a
notification; valid notifications remain silent. The bridge does not retry a
failed HTTP action automatically.

HTTP tool failures follow the supported MCP versions' [tool error
format](https://modelcontextprotocol.io/specification/2025-11-25/server/tools#error-handling).
Invalid request frames use [JSON-RPC 2.0 error
codes](https://www.jsonrpc.org/specification#error_object), with MCP's
[request-ID contract](https://modelcontextprotocol.io/specification/2025-11-25/basic#requests).

## Tests

From the repository root:

```sh
node --test packages/uma-sim-mcp/tests/mcp-handshake.test.mjs
```

The tests spawn the actual stdio bridge. They cover protocol negotiation,
notifications, string/integer request IDs, malformed requests, argument rejection
without backend calls, HTTP failures, and unchanged successful tool/resource
payloads. HTTP tests use an ephemeral local fixture server; the unreachable-backend
checks use a closed port. No Rust build or running career server is required.

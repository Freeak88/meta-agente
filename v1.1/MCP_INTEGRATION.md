# MCP/Skills/Tools Integration v1.1

## Scope

v1.1 connects Meta-Agente to external tools through three layers:

1. Minimal MCP client
2. OpenAPI bridge
3. Skill registry

This first slice implements only the minimal MCP client contract.

## Stdio Transport

The second slice adds a real stdio transport:

- Spawns an MCP server process with stdin/stdout piped
- Sends one JSON-RPC request per line
- Reads one JSON-RPC response per line
- Validates response ids
- Reuses the existing `McpClient<T>` through the `McpTransport` trait

Tests spawn a real child process that speaks mock MCP JSON-RPC over stdio,
so the transport is validated without depending on `npx` or a third-party
server.

## Minimal MCP Client

The client speaks JSON-RPC 2.0 messages compatible with MCP-style methods:

- `initialize`
- `tools/list`
- `tools/call`

The client is transport-agnostic. Tests use an in-memory mock MCP server, so the
contract validates protocol behavior without requiring a real process, socket,
or external dependency.

## Invariants

1. The client never mutates runtime `State`.
2. The client does not touch the Axum API surface.
3. The first implementation is transport-agnostic.
4. Tool listing and execution are validated through contract tests.
5. OpenAPI bridge and skill registry remain out of this first slice.

## Contract Tests

- Connect to a mock MCP server through `initialize`
- List tools through `tools/list`
- Execute one tool through `tools/call`
- Return server errors explicitly

## Deferred

- stdio transport
- HTTP/SSE transport
- OpenAPI bridge
- Skill registry import
- Mapping MCP tools into `CapabilityAdapter`

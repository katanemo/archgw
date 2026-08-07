# Plano Routing API — Request & Response Format

## Overview

Plano intercepts LLM requests and routes them to the best available model based on semantic intent and live cost/latency data. The developer sends a standard OpenAI-compatible request with an optional `routing_preferences` field. Plano returns an ordered list of candidate models; the client uses the first and falls back to the next on 429 or 5xx errors.

---

## Request Format

Standard OpenAI chat completion body. The only addition is the optional `routing_preferences` field, which is stripped before the request is forwarded upstream.

```json
POST /v1/chat/completions
{
  "model": "openai/gpt-4o-mini",
  "messages": [
    {"role": "user", "content": "write a sorting algorithm in Python"}
  ],
  "routing_preferences": [
    {
      "name": "code generation",
      "description": "generating new code snippets",
      "models": ["anthropic/claude-sonnet-4-6", "openai/gpt-4o", "openai/gpt-4o-mini"]
    },
    {
      "name": "general questions",
      "description": "casual conversation and simple queries",
      "models": ["openai/gpt-4o-mini"]
    }
  ]
}
```

### `routing_preferences` fields


| Field         | Type     | Required | Description                                                                                 |
| ------------- | -------- | -------- | ------------------------------------------------------------------------------------------- |
| `name`        | string   | yes      | Route identifier. Must match the LLM router's route classification.                         |
| `description` | string   | yes      | Natural language description used by the router to match user intent.                       |
| `models`      | string[] | yes      | Ordered candidate pool. At least one entry required. Must be declared in `model_providers`. |


### Notes

- `routing_preferences` is **optional**. If omitted, the config-defined preferences are used.
- If provided in the request body, it **overrides** the config for that single request only.
- `model` is still required and is used as the fallback if no route is matched.

---

## Response Format

```json
{
  "models": [
    "anthropic/claude-sonnet-4-6",
    "openai/gpt-4o",
    "openai/gpt-4o-mini"
  ],
  "route": "code generation",
  "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736"
}
```

### Fields


| Field      | Type          | Description                                                                                             |
| ---------- | ------------- | ------------------------------------------------------------------------------------------------------- |
| `models`   | string[]      | Ranked model list. Use `models[0]` as primary; retry with `models[1]` on 429/5xx, and so on.            |
| `route`    | string | null | Name of the matched route. `null` if no route matched — client should use the original request `model`. |
| `trace_id` | string        | Trace ID for distributed tracing and observability.                                                     |


---

## Client Usage Pattern

```python
response = plano.routing_decision(request)
models = response["models"]

for model in models:
    try:
        result = call_llm(model, messages)
        break  # success — stop trying
    except (RateLimitError, ServerError):
        continue  # try next model in the ranked list
```

---

## Configuration (set by platform/ops team)

Requires `version: v0.4.0` or above. Models listed under `routing_preferences` must be declared in `model_providers`.

```yaml
version: v0.4.0

model_providers:
  - model: anthropic/claude-sonnet-4-6
    access_key: $ANTHROPIC_API_KEY
  - model: openai/gpt-4o
    access_key: $OPENAI_API_KEY
  - model: openai/gpt-4o-mini
    access_key: $OPENAI_API_KEY
    default: true

routing_preferences:
  - name: code generation
    description: generating new code snippets or boilerplate
    models:
      - anthropic/claude-sonnet-4-6
      - openai/gpt-4o

  - name: general questions
    description: casual conversation and simple queries
    models:
      - openai/gpt-4o-mini
      - openai/gpt-4o
```

---

## Agentic Loops

A coding agent answers one user turn with many LLM calls: the model replies with tool calls, the client runs them and posts the results back for the next step. Re-routing each of those steps would let a single turn drift across models mid-generation, which breaks the client — tool-call ids, reasoning state and the provider's prompt cache all belong to the model that started the turn.

Plano detects these steps and replays the decision the loop already made instead of routing again. It works out of the box, with no client changes, for every supported client API — Anthropic `tool_result` blocks, OpenAI Chat `role: "tool"` messages, and Responses `function_call_output` items:

```text
user message      -> routing picks a model
tool result       -> replays it (routing skipped)
tool result       -> replays it (routing skipped)
next user message -> routing runs again, free to pick a different model
```

Routing is skipped only while a loop is in flight. A new user message always re-routes, so intent-based selection keeps working turn to turn. A step also re-routes when the loop can no longer be identified — the session went cold, the system prompt or tool set changed, or the request is on a different model (Claude Code's `ANTHROPIC_SMALL_FAST_MODEL` calls route independently of the main loop).

Skips are observable: the `plano.routing.skipped` span attribute and the `brightstaff_router_skips_total` counter.

## Model Affinity

Loop detection covers a single turn. To keep a whole *conversation* on one model — across user turns, or when your client's requests don't look like a tool loop — send an `X-Model-Affinity` header. Without it, Plano derives an implicit session key from the stable prompt prefix (system + tools + first user message), which is what makes loop replay work header-free.

```json
POST /v1/chat/completions
X-Model-Affinity: a1b2c3d4-5678-...

{
  "model": "openai/gpt-4o-mini",
  "messages": [...]
}
```

The routing decision endpoint also supports model affinity:

```json
POST /routing/v1/chat/completions
X-Model-Affinity: a1b2c3d4-5678-...
```

Response when pinned:

```json
{
  "models": ["anthropic/claude-sonnet-4-6"],
  "route": "code generation",
  "trace_id": "...",
  "session_id": "a1b2c3d4-5678-...",
  "pinned": true
}
```

`pinned` reports that the session was warm on its bound model, so callers can treat it as a signal to keep that provider's cache warm.

The decision endpoint applies the same loop handling as the proxy, so a client polling it between tool calls gets a stable answer for the whole turn.

Configure TTL and cache size:

```yaml
routing:
  session_ttl_seconds: 600    # default: 10 min
  session_max_entries: 10000  # upper limit
```

---

## Version Requirements


| Version    | Top-level `routing_preferences`        |
| ---------- | -------------------------------------- |
| `< v0.4.0` | Not allowed — startup error if present |
| `v0.4.0+`  | Supported (required for model routing) |

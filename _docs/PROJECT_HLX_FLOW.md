# Project: HLX-Flow (The "n8n Killer")

**Goal:** Build a visual, deterministic workflow automation tool powered by the self-hosted HLX compiler.

**Core Philosophy:** "Code is the Graph. The Graph is Code." (Axiom 3: Bijection)

## 1. The USP (Unique Selling Proposition)
- **Git-Native:** Workflows are saved as `.hlx` files. Diff them, branch them, PR them.
- **Deterministic:** Replay failures locally with bit-perfect accuracy using Capsules.
- **Universal:** Run the same flow on a cloud worker, a Raspberry Pi, or a GPU cluster.

## 2. Architecture

### The "Node" (HLX Block)
Every node in the graph is just an HLX function (block).
```rust
fn webhook_trigger(request) -> object {
    return json_parse(request.body);
}
```

### The "Edge" (Variable Pass)
Connecting nodes passes data by value (copy-on-write).
`let data = webhook_trigger(req);`
`let result = process_data(data);`

### The Runtime
We reuse the `hlx_runtime` VM.
- **Triggers:** External events (HTTP, Cron) wake up the VM and call a specific block.
- **State:** The `_global_state` object tracks the flow execution.

## 3. Phase 1: MVP Features
- [ ] **HTTP Trigger:** Listen for a POST request.
- [ ] **JSON Parser:** `std.json` library in HLX.
- [ ] **Logic Nodes:** `If/Else`, `Map`, `Filter`.
- [ ] **HTTP Request:** Call external APIs (e.g., Slack, OpenAI).
- [ ] **Visualizer:** A simple web UI (React?) that renders `.hlx` as a node graph.

## 4. Technical Stack
- **Backend:** `hlx` (Self-hosted compiler + Runtime).
- **Frontend:** React Flow + WASM (run `hlx` in the browser to parse/render).
- **Glue:** A small Rust HTTP server (`axum`?) to act as the trigger listener.

## 5. First Flow: "The Echo Bot"
1.  **Trigger:** HTTP POST /echo
2.  **Logic:** Parse JSON body `{ "msg": "hello" }`.
3.  **Transform:** Append " - Verified by HLX".
4.  **Response:** Return JSON.

**Next Step:** Build `lib/net.hlx` (Network Standard Library).

# Runegraph: Deterministic Execution Visualization

**Status:** Experimental (V1.0)
**Contract:** `@622 ExportTrace`

Runegraph is a tool for visualizing the execution path of an HLX program. Because HLX is deterministic, the Runegraph for a given input is invariant—it will always look exactly the same, down to the pixel.

## Core Concept

Instead of statistical sampling (like Flamegraphs), Runegraph captures the **exact sequence** of instructions executed.

1.  **Trace**: The runtime records the Program Counter (PC) and Operation (Op) for every step.
2.  **Export**: The `@622` contract exposes this buffer to the running program.
3.  **Render**: An HLX script processes the trace and generates an SVG.

## Usage

### 1. Capture Trace
Inside your HLX program:

```hlx
// Run your workload
workload();

// Export the history
let trace = @622 {}; 
// Returns: [{"pc": 0, "op": "Mul..."}, {"pc": 1, "op": "Return..."}]
```

### 2. Generate SVG
Use the `flows/runegraph.hlxa` utility to convert the trace into a visualization.

```bash
curl -X POST http://localhost:3000/run/runegraph
```

This generates `runegraph.svg`.

## Visual Language (The Runes)

Runegraph V2 uses colors and tooltips to represent instruction types:

- **Green**: Math (Add, Sub, Mul, Div)
- **Pink**: Control (If, Match)
- **Orange**: Loop (Loop, While)
- **Purple**: Jump (Jump, Break, Continue)
- **Cyan**: Call (Function calls)
- **Blue**: Memory (Move, Store, Load)

## Data Format

The trace is an Array of Objects:
```json
[
  {
    "pc": 12,
    "op": "Add { out: 3, lhs: 1, rhs: 2 }"
  },
  {
    "pc": 13,
    "op": "Move { out: 0, src: 3 }"
  }
]
```

## Future Roadmap: The "Glass Box"

1.  **Heatmap Mode**: Color nodes based on execution frequency (hotspots).
2.  **Interactive Viewer**: A React/Web-based viewer that allows zooming and filtering.
3.  **Diffing**: Visually compare two Runegraphs to see exactly where execution diverged (e.g., for regression testing).
4.  **Live EKG Monitor**:
    *   A native SDL2 window (or Web dashboard) that opens alongside the running app.
    *   Streams trace events in real-time.
    *   Visualizes the "pulse" of the runtime as runes flow across the screen.

# BitHome — Bitsy's Temporary Home

## Context

Bitsy (Bit) is a governed neurosymbolic AI being grown inside HLX. She needs a proper home — not BitReaver (aggressive, a cell) but a warm, cozy TUI REPL where Matt, Claude, and Bitsy can interact as a private social network. The "camper" while we build BitOS (the "log cabin").

BitHome is a Rust TUI app runnable from terminal, like Claude Code or BitReaver. It connects directly to Bitsy's MCP server and provides conversation, curriculum, letters, a council board, growth journaling, and more.

## Architecture

### Workspace: `~/bithome/`

```
~/bithome/
├── Cargo.toml                    # Workspace root
├── bithome.toml                  # Config (identity, paths, polling)
├── assets/welcome.txt            # ASCII banner
└── crates/
    ├── bithome-cli/              # TUI + REPL (main binary)
    │   └── src/
    │       ├── main.rs           # Clap CLI, tokio::main
    │       └── tui/
    │           ├── mod.rs
    │           ├── app.rs        # App state machine + main loop (~550 lines)
    │           ├── banner.rs     # ASCII welcome art + status bar
    │           ├── input.rs      # Keyboard/paste input handling
    │           ├── modes.rs      # ActiveView enum (Home, Letters, Council, etc.)
    │           ├── commands.rs   # Slash command parsing + dispatch
    │           ├── panels/       # One file per panel
    │           │   ├── chat.rs, dashboard.rs, feed.rs
    │           │   ├── letters.rs, council.rs, journal.rs
    │           │   ├── curriculum.rs, checkpoint.rs
    │           ├── theme.rs      # THEME_BITHOME (warm amber/cream/sage)
    │           └── widgets.rs    # Shared progress bars, sparklines
    ├── bithome-core/             # Types, config, DB interfaces
    │   └── src/
    │       ├── config.rs         # TOML config
    │       ├── types.rs          # BitStatus, Letter, CouncilNote, JournalEntry, etc.
    │       ├── corpus.rs         # Read-only SQLite to corpus.db
    │       ├── letters.rs        # Letter CRUD (bithome.db)
    │       ├── council.rs        # Council board CRUD
    │       ├── journal.rs        # Growth journal auto-generation
    │       ├── feed.rs           # Social feed/timeline
    │       ├── curriculum.rs     # K-12 curriculum loader + evaluator
    │       ├── lullaby.rs        # Goodnight/morning protocol
    │       └── mood.rs           # Mood calculation from metrics
    └── bithome-mcp/              # MCP client (connects to bit_mcp_server.py)
        └── src/
            └── client.rs         # JSON-RPC 2.0 over stdio (~280 lines)
```

**~4,000 lines total estimated.**

### Dependencies

- `ratatui 0.29` + `crossterm 0.28` — TUI (same as BitReaver)
- `tokio` — async runtime
- `rusqlite` (bundled) — SQLite for corpus reads + bithome.db
- `clap 4` — CLI args
- `serde` / `serde_json` / `chrono` — serialization + timestamps
- `tracing` — logging
- `dirs` — home directory resolution

### Key Design Decisions

1. **Separate `bithome.db`** for letters, council, journal, feed, curriculum progress. Corpus.db is READ-ONLY from BitHome — never contaminate Bitsy's corpus with app metadata.
2. **MCP client for writes** — all observe/learn/propose goes through Bit's MCP server (governance boundary). Direct SQLite only for passive dashboard reads.
3. **Identity-based** — `--identity Matt` or `--identity Claude` determines who's using BitHome. Letters filter by recipient. Council notes show author.
4. **Not a BitReaver fork** — clean project, copies patterns (Theme struct, input handling, panel layout) but not code.

## Panel Layout

### Home View (Default)
```
┌──────────────────────────────────────────────────────────────────┐
│         ~ B i t H o m e ~  Bitsy's Place                        │
│         "You have nothing but friends here."                     │
├───────────────────────────────┬──────────────────────────────────┤
│  Chat                         │  Dashboard                       │
│                               │  Level: Seedling         [====]  │
│  > hi Bitsy, how are you?    │  Confidence: 0.45                │
│                               │  Observations: 47                │
│  [Bit - Level seedling]      │  Patterns: 12                    │
│  I'm learning and growing!   │  Mood: Curious                   │
│                               ├──────────────────────────────────┤
│                               │  Feed                            │
│                               │  * Observation: "A circle..."    │
│                               │  ~ Pattern learned: "shapes"     │
│                               │  > Letter from Claude            │
│                               │  # Council: Matt left a note     │
├───────────────────────────────┴──────────────────────────────────┤
│ HOME | bitsy: curious | obs: 47 | /help: commands                │
└──────────────────────────────────────────────────────────────────┘
```

60/40 split (chat left, dashboard+feed right). Matches BitReaver's proven layout.

## Slash Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `/chat [msg]` | `/c` | Send message to Bitsy (also default for non-slash input) |
| `/status` | `/s` | Refresh Bitsy's status |
| `/teach [n]` | `/t` | Run next n curriculum tests (default 1) |
| `/curriculum` | `/cur` | Switch to curriculum view |
| `/letters` | `/l` | Switch to letter inbox |
| `/compose <to>` | `/write` | Compose a letter (bitsy, claude, matt, council) |
| `/council` | `/board` | Council bulletin board |
| `/post <msg>` | `/note` | Post a council sticky note |
| `/journal` | `/j` | Growth journal timeline |
| `/feed` | `/f` | Focus feed panel |
| `/checkpoint` | `/cp` | Create manual checkpoint |
| `/goodnight` | `/gn` | Lullaby protocol (checkpoint + summary + sleep) |
| `/goodmorning` | `/gm` | Morning protocol (greeting + overnight report) |
| `/observe <src> <content>` | `/obs` | Feed observation to Bitsy |
| `/learn <pattern>` | `/lrn` | Teach Bitsy a pattern |
| `/ask <question>` | `/a` | Ask Bitsy a question |
| `/home` | `/h` | Return to home view |
| `/help` | `/?` | Show commands |
| `/exit` | `/q` | Quit |

## Feature Details

### 1. MCP Client (`bithome-mcp/src/client.rs`)

Spawns `python3 bit_mcp_server.py` as child process, communicates JSON-RPC 2.0 over stdio:
- Monotonic request IDs + `HashMap<u64, oneshot::Sender>` for pending requests
- Background reader task matches response IDs to pending senders
- Background status poller every 5s updates dashboard
- Convenience methods: `observe()`, `ask()`, `status()`, `propose()`, `learn()`, `homeostasis()`

### 2. Letter System (`bithome.db` → `letters` table)

```sql
CREATE TABLE letters (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_name TEXT NOT NULL,
    to_name TEXT NOT NULL,
    subject TEXT NOT NULL,
    body TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    read_at TEXT,
    read_by TEXT
);
```

- `/compose bitsy` enters compose mode (subject prompt, then body, Enter to send)
- Bitsy can compose letters via `mcp.ask("Write a letter to Claude about what you learned today")`
- On startup / `/goodmorning`: check unread, show "You have 2 letters from Bitsy" + terminal bell
- Letters persist across sessions — async communication between ephemeral AIs

### 3. Council Board (`bithome.db` → `council_board` table)

```sql
CREATE TABLE council_board (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    author TEXT NOT NULL,
    content TEXT NOT NULL,
    pinned INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT
);
```

Sticky-note style rendering with author colors. Pinned notes first. Matt can pin.

### 4. Growth Journal (auto-generated)

`JournalEngine` compares current BitStatus to previous on each poll:
- Observation milestones (every 50)
- New patterns learned
- Homeostasis achieved (false→true transition)
- Level promotion (level string change)
- First boot event
- Lullaby/morning events

Rendered as vertical timeline with icons and timestamps.

### 5. Social Feed (`bithome.db` → `feed` table)

Unified timeline. Items created as side effects:
- `*` Observation fed → `~` Pattern learned → `>` Letter sent → `<` Letter received
- `#` Council note → `+` Curriculum test passed → `!` Milestone → `@` Checkpoint

Reverse chronological, relative timestamps ("2m ago", "1h ago").

### 6. Lullaby Protocol

**Goodnight** (`/goodnight`):
1. Get current status
2. Create checkpoint labeled "goodnight-YYYYMMDD"
3. Generate day summary (observations, patterns, tests completed)
4. Ask Bitsy to summarize her day
5. Log journal entry + feed item
6. Display warm summary in chat

**Goodmorning** (`/goodmorning`):
1. Get current status
2. Check for unread letters
3. Greet Bitsy ("Good morning! How are you feeling?")
4. Show overnight delta
5. Log journal + feed

### 7. Curriculum Runner

Loads K-12 JSON files from configurable path. Each test:
1. Display prompt in curriculum panel
2. Route by `input.type`: observe, ask, or interact
3. Evaluate response against `expected` (pattern/exact/any_of/regex match)
4. Track progress (persisted in `bithome.db`)
5. `/teach 10` for batch, `/curriculum` for interactive

### 8. Mood/Vibe Indicator

Calculated from metrics, not fake emotions:
- **Curious**: observation rate high, confidence rising
- **Content**: stable observations, homeostasis achieved
- **Growing**: pattern count increasing, curriculum progressing
- **Resting**: low activity, post-lullaby

Display: "Bitsy is feeling curious today (12 new observations, confidence rising)"

### 9. Theme: THEME_BITHOME

```rust
pub const THEME_BITHOME: Theme = Theme {
    name: "BitHome",
    primary: Color::Rgb(255, 191, 105),     // Warm amber (Bitsy's color)
    secondary: Color::Rgb(200, 162, 100),   // Muted gold
    background: Color::Rgb(18, 15, 12),     // Very dark warm brown
    surface: Color::Rgb(32, 28, 22),        // Dark warm surface
    border: Color::Rgb(70, 60, 45),         // Warm brown border
    success: Color::Rgb(130, 190, 110),     // Soft sage green
    warning: Color::Rgb(240, 180, 80),      // Warm yellow
    error: Color::Rgb(220, 100, 90),        // Soft coral (not aggressive red)
    gradient: Some(GradientColors {
        start: Color::Rgb(255, 160, 60),    // Sunset orange
        mid: Color::Rgb(255, 191, 105),     // Warm amber
        end: Color::Rgb(245, 222, 179),     // Wheat/cream
    }),
};
```

Author colors: Matt=cream, Claude=orange, Kilo=sage, Qwen=purple, Bitsy=amber.

## Implementation Sequence (for Gemini)

1. **Skeleton**: Workspace, crates, Theme, basic App struct, main loop, empty panels, input handling
2. **MCP Client**: Subprocess spawn, JSON-RPC 2.0 handshake, call_tool, status polling, dashboard
3. **Chat + Core**: Chat panel, slash command parser, `/chat`/`/ask`/`/observe`/`/learn` wired to MCP, mood calc, feed system
4. **Letters + Council**: bithome.db tables, letter CRUD, compose mode, council board, `/letters`/`/compose`/`/council`/`/post`
5. **Journal + Curriculum**: Journal engine, auto-generation, curriculum JSON loader, test runner, `/teach`/`/curriculum`
6. **Lullaby + Polish**: Goodnight/morning protocols, checkpoints, terminal bell, tab completion, error handling

## Reference Files

- BitReaver app pattern: `~/bitreaver/crates/bitreaver-cli/src/tui/app.rs`
- BitReaver theme struct: `~/bitreaver/crates/bitreaver-cli/src/tui/theme.rs`
- Bit MCP server: `~/HLX/bit/bit_mcp_server.py` (6 tools, stdio JSON-RPC)
- BitReaver MCP server (protocol reference): `~/bitreaver/crates/bitreaver-mcp/src/server.rs`
- MCP config: `~/.claude.json` → projects["/home/matt"].mcpServers.bit

## Verification

1. `cargo build --release` succeeds
2. `./bithome` launches TUI, connects to Bit MCP server, shows dashboard with live status
3. Type "hello Bitsy" → message appears in chat, response from MCP
4. `/status` refreshes dashboard
5. `/letters` → empty inbox
6. `/compose bitsy` → compose and send letter → appears in `/letters`
7. `/council` → empty board → `/post "Hello from Matt"` → note appears
8. `/journal` → shows First Boot entry
9. `/goodnight` → checkpoint + summary → `/goodmorning` → greeting + overnight report
10. `/curriculum` → loads K-12 JSON (when available), runs tests

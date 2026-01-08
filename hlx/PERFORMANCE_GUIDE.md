# HLX Performance Engineering Guide

**For Mere Mortals™** - Simple tools to find bottlenecks without needing Claude-level intuition.

## Quick Start

```bash
# Build release version first
cargo build --release

# Quick benchmark any file
./perf_toolkit.sh quick examples/fibonacci.hlxa

# Full analysis
./perf_toolkit.sh all examples/showcase_math.hlxa

# Run regression suite (checks all examples)
./perf_toolkit.sh regression
```

## The Tools & What They're Good For

### 1. **Quick Bench** (Hyperfine) 🏃
**When:** You changed something and want to know "is it faster?"
**What it does:** Runs your code 10+ times, gives you average/min/max times with confidence intervals
**Why it's good:** Eliminates measurement noise, shows you REAL differences

```bash
./perf_toolkit.sh quick examples/factorial.hlxa
```

Output:
```
Benchmark 1: hlx compile factorial.hlxa
  Time (mean ± σ):      23.4 ms ±   1.2 ms    [User: 19.8 ms, System: 3.4 ms]
  Range (min … max):    21.7 ms …  26.3 ms    10 runs
```

**Read it like:** "On average takes 23.4ms, usually between 21.7-26.3ms"

---

### 2. **CPU Profiling** (perf + flamegraph) 🔥
**When:** "Where is my code spending time?"
**What it does:** Samples your program 999 times per second, shows which functions eat CPU
**Why it's good:** Visual flamegraphs make it OBVIOUS what's slow

```bash
./perf_toolkit.sh profile examples/showcase_math.hlxa
```

Output: `flamegraph.svg` - open in browser

**How to read a flamegraph:**
- **Width** = % of time spent (wider = slower)
- **Height** = call stack depth (bottom = entry point, top = leaf function)
- **Color** = random (just for contrast)
- **Click** = zoom into that function

Look for:
- Wide boxes at the top = hot functions (optimize these!)
- Unexpected wide boxes = "why is THAT taking so long??"

---

### 3. **Memory Profiling** (Valgrind Massif) 💾
**When:** "Is my code leaking? How much RAM does it use?"
**What it does:** Tracks every allocation, shows peak memory and who allocated it
**Why it's good:** Catches leaks, shows memory growth over time

```bash
./perf_toolkit.sh memory examples/primes.hlxa
```

Output:
```
Peak memory: 4.5 MB
Definitely lost: 0 bytes (good!)
Possibly lost: 0 bytes (good!)
```

**Red flags:**
- "definitely lost" > 0 = memory leak
- Peak memory way higher than expected = allocating too much
- Slow compile = check this (memory thrashing kills perf)

---

### 4. **Cache Analysis** (Cachegrind) 🎯
**When:** "Code seems slow but flamegraph doesn't show obvious hotspots"
**What it does:** Simulates CPU cache, counts cache misses
**Why it's good:** Cache misses = waiting for RAM (100x slower than L1 cache!)

```bash
./perf_toolkit.sh cache examples/fibonacci.hlxa
```

Output:
```
I   refs:      12,450,892  (instruction reads)
I1  misses:        42,511  (L1 instruction cache misses)
LLi misses:         1,204  (last-level instruction cache misses)
D   refs:       5,123,445  (data reads/writes)
D1  misses:       128,004  (L1 data cache misses)
LLd misses:        12,391  (last-level data cache misses)
```

**Rule of thumb:**
- L1 miss rate > 5% = data structure layout problem
- LL (last level) miss rate > 1% = thrashing between functions/modules
- High D1 misses = poor data locality (arrays better than pointers)

---

### 5. **Phase Breakdown** ⏱️
**When:** "Which part of the compiler is slow? Parse? Codegen?"
**What it does:** Times each compilation phase separately
**Why it's good:** Tells you WHERE to optimize

```bash
./perf_toolkit.sh phases examples/factorial.hlxa
```

Output:
```
Parse:    8.2ms  (35%)
Lower:    3.1ms  (13%)
Codegen: 12.4ms  (52%)
Total:   23.7ms
```

**Read it like:** "Codegen is the bottleneck (52%), optimize that first"

---

### 6. **Compare** 🥊
**When:** "Which implementation is faster?"
**What it does:** Benchmarks two files side-by-side with statistical comparison
**Why it's good:** Answers "is this ACTUALLY better?" with numbers

```bash
./perf_toolkit.sh compare examples/factorial.hlxa examples/fibonacci.hlxa
```

Output:
```
Benchmark 1: factorial.hlxa
  Time: 23.4 ms ± 1.2 ms

Benchmark 2: fibonacci.hlxa
  Time: 28.7 ms ± 1.5 ms

Summary:
  factorial.hlxa is 1.23x faster
```

---

### 7. **Regression Suite** 📊
**When:** "Did my optimization break anything? Is the compiler slower now?"
**What it does:** Benchmarks ALL examples, saves results for comparison
**Why it's good:** Catches performance regressions before they ship

```bash
./perf_toolkit.sh regression
```

Output: `regression_suite.json` - compare with previous runs

**Pro tip:** Run this before and after big changes, diff the JSON files

---

## Performance Engineering Workflow

### 1. Establish Baseline
```bash
# First, know where you stand
./perf_toolkit.sh regression
cp perf_results_*/regression_suite.json baseline.json
```

### 2. Identify Bottleneck
```bash
# Profile the slow example
./perf_toolkit.sh profile examples/slow_thing.hlxa
# Look at flamegraph - what's wide?
```

### 3. Dig Deeper
```bash
# If flamegraph doesn't show obvious hotspot:
./perf_toolkit.sh cache examples/slow_thing.hlxa  # Check cache misses
./perf_toolkit.sh memory examples/slow_thing.hlxa # Check allocations
./perf_toolkit.sh phases examples/slow_thing.hlxa # Which phase?
```

### 4. Optimize & Verify
```bash
# Make your change, then:
./perf_toolkit.sh quick examples/slow_thing.hlxa  # Is it faster?

# Check you didn't break other things:
./perf_toolkit.sh regression
# Compare with baseline.json
```

### 5. Document
```bash
# Save your results
./perf_toolkit.sh compare examples/old_version.hlxa examples/new_version.hlxa
# Commit the flamegraphs and benchmark results
```

---

## Common Patterns & Solutions

### Pattern: "Parser is slow on large files"
**Symptoms:**
- Phase breakdown shows Parse taking >50% of time
- Flamegraph shows wide boxes in parser functions

**Likely causes:**
- Excessive backtracking in nom parsers
- Allocating on every parse node
- No buffering of input

**Tools to use:**
1. `./perf_toolkit.sh profile` - see which parser functions are hot
2. `./perf_toolkit.sh memory` - check if allocations are the issue
3. Check flamegraph for tall stacks (= deep recursion, possibly backtracking)

---

### Pattern: "Codegen is slow"
**Symptoms:**
- Phase breakdown shows Codegen >60%
- Memory usage spikes during codegen

**Likely causes:**
- O(n²) algorithms in instruction emission
- Repeated map lookups
- Vec reallocations (no capacity reserve)

**Tools to use:**
1. `./perf_toolkit.sh profile` - which codegen function is hot?
2. `./perf_toolkit.sh cache` - are we cache-missing on data structures?
3. Add instrumentation to count operations (loops, lookups, etc.)

---

### Pattern: "Small files are fast, big files are REALLY slow"
**Symptoms:**
- Small examples: 20ms
- 2x size example: 200ms (not 40ms!)
- Flamegraph shows same functions but way wider

**This is O(n²) behavior!**

**How to find it:**
```bash
# Create test files of increasing size
echo "Testing scalability..."
./perf_toolkit.sh quick small_10_lines.hlxa
./perf_toolkit.sh quick medium_100_lines.hlxa
./perf_toolkit.sh quick large_1000_lines.hlxa

# If 10x size = 100x time, you have O(n²)
# Profile the large one to find the quadratic loop
./perf_toolkit.sh profile large_1000_lines.hlxa
```

**Common O(n²) culprits:**
- Nested loops over same data
- Linear search in a vector (use HashMap!)
- String concatenation in a loop (use StringBuilder!)
- Repeated full-file scans

---

## Tool Installation (One-Time Setup)

```bash
# Hyperfine (benchmarking)
cargo install hyperfine

# Perf (Linux CPU profiling) - usually pre-installed
sudo pacman -S perf  # Arch
sudo apt install linux-tools-generic  # Ubuntu

# Flamegraph scripts
git clone https://github.com/brendangregg/FlameGraph
sudo cp FlameGraph/*.pl /usr/local/bin/

# Valgrind (memory profiling)
sudo pacman -S valgrind  # Arch
sudo apt install valgrind  # Ubuntu
```

---

## Reading Perf Output Like a Pro

### CPU Stats (perf stat)
```
Performance counter stats for './hlx compile test.hlxa':

        45.67 msec task-clock                #    0.945 CPUs utilized
          234      context-switches          #    5.124 K/sec
            2      cpu-migrations            #   43.800 /sec
        1,234      page-faults               #   27.015 K/sec
  123,456,789      cycles                    #    2.703 GHz
  234,567,890      instructions              #    1.90  insn per cycle
   45,678,901      branches                  #  1000.234 M/sec
      567,890      branch-misses             #    1.24% of all branches
```

**What to look for:**
- **Instructions per cycle < 1.0** = CPU is stalling (waiting for memory/cache)
- **Branch misses > 3%** = unpredictable branches (try to make them more predictable)
- **Context switches > 1000** = system is thrashing (unlikely for our use case)
- **Page faults > 10K** = memory allocation overhead

---

## Emergency Debug: "It's Slow and I Don't Know Why"

Run this:
```bash
./perf_toolkit.sh all examples/the_slow_thing.hlxa
```

Then open the flamegraph and look for:

1. **One super wide box** = that's your bottleneck, optimize that function
2. **Many thin boxes** = death by a thousand cuts, need to optimize many things
3. **Tall stack** = deep recursion, might be stack thrashing
4. **Tiny boxes repeated** = function call overhead, try inlining

Then check memory report for:
- **Peak > 100MB** = probably allocating too much
- **Leaks > 0** = memory leak, fix immediately

Then check cache report for:
- **L1 miss rate > 5%** = data layout problem
- **LL miss rate > 2%** = working set doesn't fit in cache

---

## CI Integration (TODO)

```yaml
# .github/workflows/perf.yml
name: Performance Regression Check

on: [pull_request]

jobs:
  perf:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Install tools
        run: cargo install hyperfine
      - name: Run baseline
        run: |
          git checkout main
          cargo build --release
          ./perf_toolkit.sh regression
          cp perf_results_*/regression_suite.json baseline.json
      - name: Run PR version
        run: |
          git checkout ${{ github.head_ref }}
          cargo build --release
          ./perf_toolkit.sh regression
          cp perf_results_*/regression_suite.json pr.json
      - name: Compare
        run: |
          # Fail if any benchmark is >10% slower
          python compare_benchmarks.py baseline.json pr.json --threshold 1.1
```

---

## Further Reading

- **Brendan Gregg's Website**: http://www.brendangregg.com/perf.html (the perf god)
- **Rust Performance Book**: https://nnethercote.github.io/perf-book/
- **What Every Programmer Should Know About Memory**: https://people.freebsd.org/~lstewart/articles/cpumemory.pdf
- **Gallery of CPU cache effects**: https://igoro.com/archive/gallery-of-processor-cache-effects/

---

## Troubleshooting

**"perf: command not found"**
- Install with `sudo pacman -S perf` (Arch) or `sudo apt install linux-tools-generic` (Ubuntu)
- On Ubuntu, you may need `linux-tools-$(uname -r)`

**"Permission denied" when running perf**
- Try: `sudo sysctl -w kernel.perf_event_paranoid=-1`
- Or run with sudo: `sudo ./perf_toolkit.sh profile test.hlxa`

**"Hyperfine not found, falling back..."**
- Install with `cargo install hyperfine`
- Basic timing still works, just less accurate

**Flamegraph is blank/broken**
- Make sure FlameGraph scripts are in PATH
- Check that perf.data was generated (should be in current dir)
- Try: `perf script | head` to verify perf data is readable

---

## Contributing Performance Improvements

When you optimize something:

1. **Benchmark before & after**
   ```bash
   ./perf_toolkit.sh compare old_version.hlxa new_version.hlxa
   ```

2. **Include flamegraphs in PR**
   - Before and after flamegraphs help reviewers understand the change

3. **Document in commit message**
   ```
   perf: optimize parser backtracking in expr parsing

   Reduced parser time by 40% on large files by eliminating
   excessive backtracking in binary operator parsing.

   Benchmark (showcase_math.hlxa):
   - Before: 45.2ms ± 2.1ms
   - After:  27.1ms ± 1.3ms
   - Speedup: 1.67x

   See flamegraph comparison in PR.
   ```

4. **Update regression baseline**
   ```bash
   ./perf_toolkit.sh regression
   git add perf_results_*/regression_suite.json
   git commit -m "perf: update regression baseline after optimization"
   ```

---

**Remember:** Measure first, optimize second.

"Premature optimization is the root of all evil." - Donald Knuth

But also: "You can't optimize what you don't measure." - Tom DeMarco

So measure everything, optimize the hot paths, and ship fast code. 🚀

#!/bin/bash
# HLX Performance Engineering Toolkit
# Usage: ./perf_toolkit.sh [command] [file]
#
# Commands:
#   quick <file>     - Quick benchmark (hyperfine)
#   profile <file>   - CPU profiling (perf + flamegraph)
#   memory <file>    - Memory profiling (valgrind massif)
#   cache <file>     - Cache analysis (cachegrind)
#   phases <file>    - Compilation phase breakdown
#   compare <f1> <f2> - Compare two files
#   regression       - Run regression suite
#   all <file>       - Run everything

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

COMPILER="./target/release/hlx"
RESULTS_DIR="perf_results_$(date +%Y%m%d_%H%M%S)"

# Check if tools are installed
check_tool() {
    if ! command -v "$1" &> /dev/null; then
        echo -e "${YELLOW}⚠ $1 not installed. Install with: $2${NC}"
        return 1
    fi
    return 0
}

# ============================================
# QUICK BENCHMARK (Hyperfine)
# ============================================
quick_bench() {
    local file="$1"
    echo -e "${BLUE}=== Quick Benchmark (Hyperfine) ===${NC}"

    if ! check_tool "hyperfine" "cargo install hyperfine"; then
        echo "Falling back to basic timing..."
        echo "Warmup runs..."
        for i in {1..3}; do
            $COMPILER compile "$file" -o /tmp/bench_output.lcc > /dev/null 2>&1
        done

        echo "Timing runs (n=10)..."
        TIMEFORMAT='%3R'
        total=0
        for i in {1..10}; do
            t=$( { time $COMPILER compile "$file" -o /tmp/bench_output.lcc > /dev/null 2>&1; } 2>&1 )
            total=$(awk "BEGIN {print $total + $t}")
        done
        avg=$(awk "BEGIN {printf \"%.3f\", $total / 10}")
        echo -e "${GREEN}Average: ${avg}s${NC}"
    else
        hyperfine --warmup 3 --runs 10 \
            --export-markdown "$RESULTS_DIR/bench_$(basename "$file").md" \
            --export-json "$RESULTS_DIR/bench_$(basename "$file").json" \
            "$COMPILER compile $file -o /tmp/bench_output.lcc"
    fi
}

# ============================================
# CPU PROFILING (perf + flamegraph)
# ============================================
cpu_profile() {
    local file="$1"
    echo -e "${BLUE}=== CPU Profiling (perf) ===${NC}"

    if ! check_tool "perf" "sudo pacman -S perf"; then
        echo "Skipping CPU profiling"
        return
    fi

    local outname=$(basename "$file" .hlx)

    echo "Recording perf data..."
    perf record -F 999 -g -- $COMPILER compile "$file" -o /tmp/perf_output.lcc 2>&1 | grep -v "Lowering"

    echo "Generating flamegraph..."
    if check_tool "perf-script" ""; then
        perf script > "$RESULTS_DIR/${outname}_perf.script"

        if check_tool "stackcollapse-perf.pl" "git clone https://github.com/brendangregg/FlameGraph"; then
            stackcollapse-perf.pl "$RESULTS_DIR/${outname}_perf.script" > "$RESULTS_DIR/${outname}_folded.txt"
            flamegraph.pl "$RESULTS_DIR/${outname}_folded.txt" > "$RESULTS_DIR/${outname}_flamegraph.svg"
            echo -e "${GREEN}✓ Flamegraph: $RESULTS_DIR/${outname}_flamegraph.svg${NC}"
        fi
    fi

    echo "CPU stats:"
    perf stat $COMPILER compile "$file" -o /tmp/perf_output.lcc 2>&1 | grep -E "instructions|cycles|cache-misses|branches"

    rm -f perf.data perf.data.old
}

# ============================================
# MEMORY PROFILING (Valgrind)
# ============================================
memory_profile() {
    local file="$1"
    echo -e "${BLUE}=== Memory Profiling (Valgrind Massif) ===${NC}"

    if ! check_tool "valgrind" "sudo pacman -S valgrind"; then
        echo "Skipping memory profiling"
        return
    fi

    local outname=$(basename "$file" .hlx)

    echo "Recording memory usage..."
    valgrind --tool=massif --massif-out-file="$RESULTS_DIR/${outname}_massif.out" \
        $COMPILER compile "$file" -o /tmp/mem_output.lcc 2>&1 | tail -5

    if check_tool "ms_print" ""; then
        ms_print "$RESULTS_DIR/${outname}_massif.out" > "$RESULTS_DIR/${outname}_memory.txt"
        echo -e "${GREEN}✓ Memory report: $RESULTS_DIR/${outname}_memory.txt${NC}"
        echo ""
        echo "Peak memory usage:"
        grep "peak" "$RESULTS_DIR/${outname}_memory.txt" | head -1
    fi

    # Also run with memcheck for leaks
    echo ""
    echo "Checking for memory leaks..."
    valgrind --leak-check=summary --error-exitcode=1 \
        $COMPILER compile "$file" -o /tmp/mem_output.lcc 2>&1 | grep -E "definitely lost|indirectly lost|ERROR SUMMARY"
}

# ============================================
# CACHE ANALYSIS (Cachegrind)
# ============================================
cache_analysis() {
    local file="$1"
    echo -e "${BLUE}=== Cache Analysis (Cachegrind) ===${NC}"

    if ! check_tool "valgrind" "sudo pacman -S valgrind"; then
        echo "Skipping cache analysis"
        return
    fi

    local outname=$(basename "$file" .hlx)

    valgrind --tool=cachegrind --cachegrind-out-file="$RESULTS_DIR/${outname}_cachegrind.out" \
        $COMPILER compile "$file" -o /tmp/cache_output.lcc 2>&1 | tail -1

    if check_tool "cg_annotate" ""; then
        cg_annotate "$RESULTS_DIR/${outname}_cachegrind.out" > "$RESULTS_DIR/${outname}_cache.txt"
        echo -e "${GREEN}✓ Cache report: $RESULTS_DIR/${outname}_cache.txt${NC}"
        echo ""
        echo "Cache miss summary:"
        grep "PROGRAM TOTALS" "$RESULTS_DIR/${outname}_cache.txt" -A 3
    fi
}

# ============================================
# PHASE BREAKDOWN
# ============================================
phase_breakdown() {
    local file="$1"
    echo -e "${BLUE}=== Compilation Phase Breakdown ===${NC}"

    # Compile with RUST_LOG=debug to get phase timings
    RUST_LOG=info time -v $COMPILER compile "$file" -o /tmp/phase_output.lcc 2>&1 | \
        grep -E "Elapsed|Parse|Lower|Codegen|Maximum resident"
}

# ============================================
# COMPARE TWO FILES
# ============================================
compare_files() {
    local f1="$1"
    local f2="$2"
    echo -e "${BLUE}=== Comparing Performance ===${NC}"
    echo "File 1: $f1 ($(wc -l < "$f1") lines)"
    echo "File 2: $f2 ($(wc -l < "$f2") lines)"
    echo ""

    if check_tool "hyperfine" "cargo install hyperfine"; then
        hyperfine --warmup 3 \
            --export-markdown "$RESULTS_DIR/comparison.md" \
            -n "$(basename "$f1")" "$COMPILER compile $f1 -o /tmp/out1.lcc" \
            -n "$(basename "$f2")" "$COMPILER compile $f2 -o /tmp/out2.lcc"
    else
        echo "Benchmarking $f1..."
        t1=$( { time $COMPILER compile "$f1" -o /tmp/out1.lcc > /dev/null 2>&1; } 2>&1 )
        echo "  Time: ${t1}s"

        echo "Benchmarking $f2..."
        t2=$( { time $COMPILER compile "$f2" -o /tmp/out2.lcc > /dev/null 2>&1; } 2>&1 )
        echo "  Time: ${t2}s"

        if (( $(echo "$t1 < $t2" | bc -l) )); then
            speedup=$(awk "BEGIN {printf \"%.2f\", $t2 / $t1}")
            echo -e "${GREEN}$f1 is ${speedup}x faster${NC}"
        else
            speedup=$(awk "BEGIN {printf \"%.2f\", $t1 / $t2}")
            echo -e "${GREEN}$f2 is ${speedup}x faster${NC}"
        fi
    fi
}

# ============================================
# REGRESSION SUITE
# ============================================
regression_suite() {
    echo -e "${BLUE}=== Regression Test Suite ===${NC}"

    local examples=(
        "examples/axiom_test.hlx"
        "examples/test_simple_math.hlx"
        "examples/factorial.hlx"
        "examples/fibonacci.hlx"
        "examples/showcase_math.hlx"
    )

    echo "Benchmarking ${#examples[@]} examples..."

    if check_tool "hyperfine" "cargo install hyperfine"; then
        local cmds=()
        for ex in "${examples[@]}"; do
            if [ -f "$ex" ]; then
                cmds+=(-n "$(basename "$ex" .hlx)" "$COMPILER compile $ex -o /tmp/reg_out.lcc")
            fi
        done

        hyperfine --warmup 2 "${cmds[@]}" \
            --export-markdown "$RESULTS_DIR/regression_suite.md" \
            --export-json "$RESULTS_DIR/regression_suite.json"

        echo -e "${GREEN}✓ Results saved to $RESULTS_DIR/${NC}"
    else
        for ex in "${examples[@]}"; do
            if [ -f "$ex" ]; then
                echo -n "$(basename "$ex" .hlx): "
                t=$( { time $COMPILER compile "$ex" -o /tmp/reg_out.lcc > /dev/null 2>&1; } 2>&1 )
                echo "${t}s"
            fi
        done
    fi
}

# ============================================
# MAIN
# ============================================

mkdir -p "$RESULTS_DIR"

case "$1" in
    quick)
        quick_bench "$2"
        ;;
    profile)
        cpu_profile "$2"
        ;;
    memory)
        memory_profile "$2"
        ;;
    cache)
        cache_analysis "$2"
        ;;
    phases)
        phase_breakdown "$2"
        ;;
    compare)
        compare_files "$2" "$3"
        ;;
    regression)
        regression_suite
        ;;
    all)
        echo -e "${GREEN}Running full performance analysis...${NC}"
        echo ""
        quick_bench "$2"
        echo ""
        cpu_profile "$2"
        echo ""
        memory_profile "$2"
        echo ""
        cache_analysis "$2"
        echo ""
        phase_breakdown "$2"
        echo ""
        echo -e "${GREEN}=== All results saved to $RESULTS_DIR ===${NC}"
        ;;
    *)
        echo "HLX Performance Engineering Toolkit"
        echo ""
        echo "Usage: $0 <command> [file]"
        echo ""
        echo "Commands:"
        echo "  quick <file>      - Quick benchmark with hyperfine"
        echo "  profile <file>    - CPU profiling + flamegraph"
        echo "  memory <file>     - Memory profiling (valgrind)"
        echo "  cache <file>      - Cache miss analysis"
        echo "  phases <file>     - Compilation phase breakdown"
        echo "  compare <f1> <f2> - Compare two files"
        echo "  regression        - Run regression suite"
        echo "  all <file>        - Run complete analysis"
        echo ""
        echo "Examples:"
        echo "  $0 quick examples/factorial.hlx"
        echo "  $0 profile examples/showcase_math.hlx"
        echo "  $0 compare examples/fibonacci.hlx examples/primes.hlx"
        echo "  $0 regression"
        exit 1
        ;;
esac

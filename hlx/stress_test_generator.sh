#!/bin/bash
# HLX Stress Test Generator
# Creates increasingly complex HLX programs to find scalability limits

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

COMPILER="./target/release/hlx"
TEST_DIR="stress_tests"

mkdir -p "$TEST_DIR"

# ============================================
# Test 1: Many Simple Functions
# ============================================
generate_many_functions() {
    local count=$1
    local file="$TEST_DIR/many_functions_${count}.hlx"

    echo "// Stress test: $count simple functions" > "$file"
    echo "program stress_test {" >> "$file"

    for i in $(seq 1 $count); do
        echo "    fn f${i}() { return ${i}; }" >> "$file"
    done

    echo "    fn main() { return f${count}(); }" >> "$file"
    echo "}" >> "$file"

    echo "$file"
}

# ============================================
# Test 2: Deep Expression Nesting
# ============================================
generate_deep_expr() {
    local depth=$1
    local file="$TEST_DIR/deep_expr_${depth}.hlx"

    echo "// Stress test: expression nesting depth $depth" > "$file"
    echo "program stress_test {" >> "$file"
    echo "    fn main() {" >> "$file"
    echo -n "        return " >> "$file"

    # Build: (((1 + 1) + 1) + 1)...
    for i in $(seq 1 $depth); do
        echo -n "(" >> "$file"
    done
    echo -n "1" >> "$file"
    for i in $(seq 1 $depth); do
        echo -n " + 1)" >> "$file"
    done
    echo ";" >> "$file"

    echo "    }" >> "$file"
    echo "}" >> "$file"

    echo "$file"
}

# ============================================
# Test 3: Long Function (Many Statements)
# ============================================
generate_long_function() {
    local statements=$1
    local file="$TEST_DIR/long_function_${statements}.hlx"

    echo "// Stress test: $statements statements in one function" > "$file"
    echo "program stress_test {" >> "$file"
    echo "    fn main() {" >> "$file"
    echo "        let result = 0;" >> "$file"

    for i in $(seq 1 $statements); do
        echo "        result = (result + ${i});" >> "$file"
    done

    echo "        return result;" >> "$file"
    echo "    }" >> "$file"
    echo "}" >> "$file"

    echo "$file"
}

# ============================================
# Test 4: Wide Array Literals
# ============================================
generate_wide_array() {
    local size=$1
    local file="$TEST_DIR/wide_array_${size}.hlx"

    echo "// Stress test: array with $size elements" > "$file"
    echo "program stress_test {" >> "$file"
    echo "    fn main() {" >> "$file"
    echo -n "        let arr = [" >> "$file"

    for i in $(seq 1 $size); do
        echo -n "$i" >> "$file"
        if [ $i -lt $size ]; then
            echo -n ", " >> "$file"
        fi
    done

    echo "];" >> "$file"
    echo "        return arr;" >> "$file"
    echo "    }" >> "$file"
    echo "}" >> "$file"

    echo "$file"
}

# ============================================
# Test 5: Deep Call Chain
# ============================================
generate_deep_calls() {
    local depth=$1
    local file="$TEST_DIR/deep_calls_${depth}.hlx"

    echo "// Stress test: call chain depth $depth" > "$file"
    echo "program stress_test {" >> "$file"

    for i in $(seq 1 $depth); do
        local next=$((i + 1))
        echo "    fn f${i}() {" >> "$file"
        if [ $i -eq $depth ]; then
            echo "        return ${i};" >> "$file"
        else
            echo "        return f${next}();" >> "$file"
        fi
        echo "    }" >> "$file"
    done

    echo "    fn main() { return f1(); }" >> "$file"
    echo "}" >> "$file"

    echo "$file"
}

# ============================================
# Test 6: Complex Generator (like LoRA example)
# ============================================
generate_complex_generator() {
    local iterations=$1
    local file="$TEST_DIR/complex_generator_${iterations}.hlx"

    echo "// Stress test: complex generator with $iterations iterations" > "$file"
    cat > "$file" << 'EOF'
program stress_test {
    fn int_to_string(n) {
        if (n == 0) { return "0"; }
        let s = "";
        let val = n;
        let i = 0;
        loop (val > 0, 1000) {
            let digit = (val - ((val / 10) * 10));
            if (digit == 0) { s = ("0" + s); }
            if (digit == 1) { s = ("1" + s); }
            if (digit == 2) { s = ("2" + s); }
            if (digit == 3) { s = ("3" + s); }
            if (digit == 4) { s = ("4" + s); }
            if (digit == 5) { s = ("5" + s); }
            if (digit == 6) { s = ("6" + s); }
            if (digit == 7) { s = ("7" + s); }
            if (digit == 8) { s = ("8" + s); }
            if (digit == 9) { s = ("9" + s); }
            val = (val / 10);
            i = (i + 1);
        }
        return s;
    }

    fn generate(count) {
        let examples = [];
        let i = 0;
        loop (i < count, DEFAULT_MAX_ITER()) {
            let id_str = int_to_string(i);
            let prompt = ("Example " + id_str);
            let code = ("fn main() { return " + id_str + "; }");
            let entry = { "id": i, "instruction": prompt, "output": code };
            examples = append(examples, entry);
            i = (i + 1);
        }
        return examples;
    }

    fn main() {
        return generate(COUNT);
    }
}
EOF

    # Replace COUNT placeholder
    sed -i "s/COUNT/${iterations}/" "$file"

    echo "$file"
}

# ============================================
# Run Test Suite
# ============================================

echo -e "${YELLOW}=== HLX Stress Test Generator ===${NC}"
echo ""
echo "Building release compiler..."
cargo build --release --bin hlx > /dev/null 2>&1
echo -e "${GREEN}✓ Compiler ready${NC}"
echo ""

# Test progressively larger inputs
echo "Generating test files..."
echo ""

# Test 1: Many Functions
echo -e "${YELLOW}Test 1: Many Functions${NC}"
for count in 10 50 100 250 500 1000; do
    file=$(generate_many_functions $count)
    printf "  %4d functions: " $count

    start=$(date +%s%N)
    if timeout 30 $COMPILER compile "$file" -o /tmp/stress_out.lcc > /dev/null 2>&1; then
        end=$(date +%s%N)
        time=$(awk "BEGIN {printf \"%.3f\", ($end - $start) / 1000000000}")
        lines=$(wc -l < "$file")
        lps=$(awk "BEGIN {printf \"%.0f\", $lines / $time}")
        echo -e "${GREEN}${time}s ($lps lines/sec)${NC}"
    else
        echo -e "${RED}TIMEOUT/FAILED${NC}"
        break
    fi
done
echo ""

# Test 2: Deep Expressions
echo -e "${YELLOW}Test 2: Deep Expression Nesting${NC}"
for depth in 10 50 100 250 500; do
    file=$(generate_deep_expr $depth)
    printf "  Depth %4d: " $depth

    start=$(date +%s%N)
    if timeout 30 $COMPILER compile "$file" -o /tmp/stress_out.lcc > /dev/null 2>&1; then
        end=$(date +%s%N)
        time=$(awk "BEGIN {printf \"%.3f\", ($end - $start) / 1000000000}")
        echo -e "${GREEN}${time}s${NC}"
    else
        echo -e "${RED}TIMEOUT/FAILED${NC}"
        break
    fi
done
echo ""

# Test 3: Long Functions
echo -e "${YELLOW}Test 3: Long Functions (Many Statements)${NC}"
for stmts in 10 50 100 250 500 1000; do
    file=$(generate_long_function $stmts)
    printf "  %4d statements: " $stmts

    start=$(date +%s%N)
    if timeout 30 $COMPILER compile "$file" -o /tmp/stress_out.lcc > /dev/null 2>&1; then
        end=$(date +%s%N)
        time=$(awk "BEGIN {printf \"%.3f\", ($end - $start) / 1000000000}")
        echo -e "${GREEN}${time}s${NC}"
    else
        echo -e "${RED}TIMEOUT/FAILED${NC}"
        break
    fi
done
echo ""

# Test 4: Wide Arrays
echo -e "${YELLOW}Test 4: Wide Array Literals${NC}"
for size in 10 50 100 250 500; do
    file=$(generate_wide_array $size)
    printf "  %4d elements: " $size

    start=$(date +%s%N)
    if timeout 30 $COMPILER compile "$file" -o /tmp/stress_out.lcc > /dev/null 2>&1; then
        end=$(date +%s%N)
        time=$(awk "BEGIN {printf \"%.3f\", ($end - $start) / 1000000000}")
        echo -e "${GREEN}${time}s${NC}"
    else
        echo -e "${RED}TIMEOUT/FAILED${NC}"
        break
    fi
done
echo ""

# Test 5: Deep Call Chains
echo -e "${YELLOW}Test 5: Deep Call Chains${NC}"
for depth in 10 50 100 250 500; do
    file=$(generate_deep_calls $depth)
    printf "  Depth %4d: " $depth

    start=$(date +%s%N)
    if timeout 30 $COMPILER compile "$file" -o /tmp/stress_out.lcc > /dev/null 2>&1; then
        end=$(date +%s%N)
        time=$(awk "BEGIN {printf \"%.3f\", ($end - $start) / 1000000000}")
        echo -e "${GREEN}${time}s${NC}"
    else
        echo -e "${RED}TIMEOUT/FAILED${NC}"
        break
    fi
done
echo ""

# Test 6: Complex Generator
echo -e "${YELLOW}Test 6: Complex Generator (LoRA-style)${NC}"
for iters in 5 10 25 50 100; do
    file=$(generate_complex_generator $iters)
    printf "  %3d iterations: " $iters

    start=$(date +%s%N)
    if timeout 60 $COMPILER compile "$file" -o /tmp/stress_out.lcc > /dev/null 2>&1; then
        end=$(date +%s%N)
        time=$(awk "BEGIN {printf \"%.3f\", ($end - $start) / 1000000000}")
        instrs=$($COMPILER inspect /tmp/stress_out.lcc 2>/dev/null | grep "Instructions:" | awk '{print $2}')
        echo -e "${GREEN}${time}s ($instrs instructions)${NC}"
    else
        echo -e "${RED}TIMEOUT/FAILED${NC}"
        break
    fi
done
echo ""

echo -e "${GREEN}=== Stress Test Complete ===${NC}"
echo "Test files saved in: $TEST_DIR/"
echo ""
echo "To profile a specific test:"
echo "  ./perf_toolkit.sh profile $TEST_DIR/some_test.hlx"

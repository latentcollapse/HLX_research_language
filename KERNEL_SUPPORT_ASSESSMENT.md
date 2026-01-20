# HLX Kernel Support Assessment
**Date**: 2026-01-20
**Status**: Self-hosting achieved, now expanding for full kernel capability
**Goal**: Make HLX bulletproof for kernel development

---

## Current State: What Works ✅

### Language Features
- ✅ Contract syntax: `{id:{@field:value}}`
- ✅ Handle operations: `collapse()` / `resolve()`
- ✅ Basic types: i64, String, arrays, bool
- ✅ Control flow: if/else, loops with bounds, break/continue
- ✅ Functions with parameters and return values
- ✅ Module system with imports/exports
- ✅ Inline assembly: `asm("...")`
- ✅ Constants: `const NAME: type = value`
- ✅ Arrays with pass-by-value semantics

### Compiler Infrastructure
- ✅ Self-hosting compiler (25K LOC in HLX)
- ✅ Native VM with 37 instruction handlers
- ✅ Axiom validators (A1-A4 enforcement)
- ✅ Type checking and inference
- ✅ Bytecode emission to LC-B format

---

## Critical Gaps for Kernel Development ❌

### 1. **Multi-Function Module Compilation** (BLOCKING)
**Issue**: When compiling a module with multiple functions, only the entry point gets emitted
**Impact**: Cannot call helper functions within kernel code
**Example**: `display_helinux()` not found even though it's in the module

**Test Case**:
```hlx
module test {
    fn main() {
        helper();  // ERROR: Unknown function
    }

    fn helper() {
        // This function doesn't get compiled into bytecode
    }
}
```

**Root Cause**: Compiler's `lower.hlx` likely only processes entry-point function
**Fix Required**: Emit ALL functions in module to bytecode, create function table

---

### 2. **Pointer Types and Operations** (HIGH PRIORITY)
**Issue**: No explicit pointer type support for raw memory access
**Impact**: Kernel needs pointer arithmetic for hardware access
**Current Workaround**: Use i64 as "address" type

**Needed**:
```hlx
fn write_memory(addr: *mut u8, value: u8) {
    *addr = value;  // Dereference and write
}

let ptr: *const u64 = 0xB8000 as *const u64;
let value = *ptr;  // Read from pointer
```

**Required Features**:
- `*const T` and `*mut T` pointer types
- Dereference operator `*ptr`
- Address-of operator `&var`
- Pointer arithmetic `ptr + offset`
- Cast from integer to pointer `addr as *mut T`

---

### 3. **Sized Integer Types** (HIGH PRIORITY)
**Issue**: Only i64 supported, kernel needs u8/u16/u32/u64 for hardware
**Impact**: Cannot properly represent hardware registers or port I/O

**Needed**:
```hlx
fn write_port_u8(port: u16, value: u8) {
    asm("outb %0, %1" : : "a"(value), "Nd"(port));
}

fn read_port_u32(port: u16) -> u32 {
    let value: u32;
    asm("inl %1, %0" : "=a"(value) : "Nd"(port));
    return value;
}
```

**Required Types**:
- `u8`, `u16`, `u32`, `u64` (unsigned)
- `i8`, `i16`, `i32`, `i64` (signed)
- Explicit casts between sizes: `value as u32`
- Zero-extend and sign-extend semantics

---

### 4. **Memory-Mapped I/O Support** (MEDIUM PRIORITY)
**Issue**: No safe abstraction for MMIO operations
**Impact**: Direct hardware access is error-prone

**Needed**:
```hlx
fn read_mmio_u32(addr: u64) -> u32 {
    let ptr = addr as *const volatile u32;
    return *ptr;  // volatile read
}

fn write_mmio_u32(addr: u64, value: u32) {
    let ptr = addr as *mut volatile u32;
    *ptr = value;  // volatile write
}
```

**Required Features**:
- `volatile` qualifier for pointers
- Guarantee no compiler optimization on volatile access
- Memory barrier support via `asm(...)`

---

### 5. **Bitfield Access** (MEDIUM PRIORITY)
**Issue**: Hardware registers use packed bitfields
**Impact**: Manual bit masking is error-prone

**Current (verbose)**:
```hlx
let access = (value >> 16) & 0xFF;
let gran = ((value >> 20) & 0x0F) | ((limit >> 16) & 0x0F);
```

**Desired**:
```hlx
bitfield GDTEntry {
    limit_low: u16 @ 0..15,
    base_low: u16 @ 16..31,
    base_mid: u8 @ 32..39,
    access: u8 @ 40..47,
    // ...
}

let entry = GDTEntry { access: 0x9A, ... };
let access_byte = entry.access;  // Extracts bits 40-47
```

---

### 6. **Interrupt Handling** (HIGH PRIORITY)
**Issue**: No interrupt descriptor table (IDT) support
**Impact**: Cannot handle hardware interrupts or exceptions

**Needed**:
```hlx
#[interrupt]
fn keyboard_handler() {
    let scancode = read_port_u8(0x60);
    // Handle keyboard input
}

#[exception]
fn page_fault_handler(error_code: u64, cr2: u64) {
    // Handle page fault
}
```

**Required Features**:
- `#[interrupt]` and `#[exception]` attributes
- Compiler-generated interrupt frame setup/teardown
- IDT structure support
- Interrupt enable/disable primitives

---

### 7. **Proper Error Handling** (MEDIUM PRIORITY)
**Issue**: No Result/Option types for safe error propagation
**Impact**: Kernel cannot handle failures gracefully

**Needed**:
```hlx
fn allocate_page() -> Result<u64, AllocError> {
    if (no_pages_available()) {
        return Err(AllocError::OutOfMemory);
    }
    return Ok(page_address);
}

// Usage
let page = allocate_page()?;  // Early return on error
```

**Required Features**:
- `Result<T, E>` type
- `Option<T>` type
- `?` operator for error propagation
- Pattern matching on Result/Option

---

### 8. **Static Memory Allocation** (LOW PRIORITY)
**Issue**: No compile-time memory allocation for static data
**Impact**: Kernel needs static buffers, stacks, page tables

**Needed**:
```hlx
static mut KERNEL_STACK: [u8; 16384] = [0; 16384];
static PAGE_TABLE: [u64; 512] = [0; 512];

fn init() {
    let stack_top = &KERNEL_STACK[16383] as u64;
    // Use stack_top
}
```

---

## Priority Roadmap

### Phase A: Core Blockers (Week 1)
1. ✅ **Multi-function modules** - MUST FIX FIRST
   - Modify `lower.hlx` to emit all functions
   - Create function lookup table in bytecode
   - Test with multi-function kernel modules

2. **Pointer types** - Essential for hardware access
   - Add `*const T` and `*mut T` to type system
   - Implement pointer arithmetic in VM
   - Add dereference operations

3. **Sized integers** - Required for register I/O
   - Add u8/u16/u32/u64 types
   - Implement casting operations
   - Test with port I/O examples

### Phase B: Hardware Access (Week 2)
4. **MMIO support** - Safe hardware register access
   - Add `volatile` keyword
   - Guarantee no optimization on volatile ops
   - Test with VGA buffer access

5. **Interrupt handling** - Enable async events
   - Design IDT structure
   - Implement `#[interrupt]` attribute
   - Create interrupt frame handling

### Phase C: Safety Features (Week 3)
6. **Error handling** - Kernel reliability
   - Implement Result<T, E> type
   - Add Option<T> type
   - Implement `?` operator

7. **Bitfields** - Clean hardware register access
   - Design bitfield syntax
   - Implement bit extraction/insertion
   - Test with GDT/IDT structures

8. **Static allocation** - Kernel data structures
   - Add `static` keyword
   - Implement compile-time initialization
   - Test with kernel stacks and tables

---

## Success Criteria

### Milestone 1: Basic Kernel (Week 1 Complete)
- [ ] Multi-function modules work
- [ ] Can write helper functions in kernel
- [ ] Pointer operations functional
- [ ] u8/u16/u32/u64 types work
- [ ] Port I/O compiles and runs

### Milestone 2: Hardware Kernel (Week 2 Complete)
- [ ] MMIO operations safe and correct
- [ ] IDT can be setup and loaded
- [ ] Basic interrupt handler works
- [ ] Keyboard input functional

### Milestone 3: Production Kernel (Week 3 Complete)
- [ ] Error handling throughout kernel
- [ ] Bitfield access for all hardware
- [ ] Static buffers for kernel data
- [ ] Complete boot sequence with GDT/IDT
- [ ] HELINUX displays via proper VGA driver

---

## Next Immediate Action

**Fix multi-function module compilation in native HLX compiler**

1. Open `/home/matt/hlx-compiler/hlx/hlx_bootstrap/lower.hlx`
2. Find where functions are lowered to bytecode
3. Modify to emit ALL functions in module, not just entry point
4. Create function name → bytecode offset lookup table
5. Test with `boot.hlx` (has multiple functions)

This is the **critical blocker** preventing full kernel development.

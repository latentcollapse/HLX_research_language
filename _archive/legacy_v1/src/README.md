# Source Code

Core Vulkan implementation for HLX runtime GPU acceleration.

## Structure

- **Rust Files** (*.rs)
  - `lib.rs` - Library entry point
  - `context.rs` - Vulkan context management
  - `buffer.rs` - GPU buffer operations
  - `pipeline.rs` - Pipeline construction and management
  - `shader.rs` - Shader compilation and management
  - `validation.rs` - Validation utilities
  - `error.rs` - Error types

- **Subdirectories**
  - `bin/` - Executable binaries
  - `shaders/` - GLSL shader source files

## Building

```bash
cargo build --release
```

## Key Components

- **Context Management** - Handles Vulkan instance, device, and queue creation
- **Pipelines** - Compute and graphics pipeline handling
- **Buffers** - GPU memory management and operations
- **Shaders** - GLSL shader compilation to SPIR-V
- **Validation** - Validation layers and error handling

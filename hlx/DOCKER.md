# HLX Docker Guide

This guide explains how to use Docker to build and run the HLX compiler and Language Server Protocol (LSP) server.

## Quick Start

### Build the Docker Image

```bash
# Build the image (takes ~10-15 minutes on first build)
docker-compose build

# Or build manually:
docker build -t hlx-compiler:latest .
```

### Run the HLX Compiler

```bash
# Show help
docker-compose run --rm hlx-compiler hlx --help

# Compile a HLX file
docker-compose run --rm hlx-compiler hlx compile examples/hello_world.hlx

# Run a HLX program
docker-compose run --rm hlx-compiler hlx run examples/hello_world.hlx
```

### Run the HLX LSP Server

```bash
# Start the LSP server
docker-compose up hlx-lsp

# Or run interactively:
docker-compose run --rm hlx-lsp
```

### Development Container

```bash
# Start an interactive development environment
docker-compose run --rm hlx-dev bash

# Inside the container:
hlx --version
hlx_lsp --help
```

---

## Docker Image Details

### Multi-Stage Build

The Dockerfile uses a multi-stage build for optimal image size:

1. **Builder Stage** (rust:1.75-bookworm)
   - Installs build dependencies (LLVM 18, SDL2, Vulkan)
   - Compiles HLX binaries in release mode
   - ~2.5 GB during build

2. **Runtime Stage** (debian:bookworm-slim)
   - Minimal runtime dependencies only
   - Non-root user for security
   - Final image: ~500 MB

### Included Binaries

- `/app/bin/hlx` - HLX compiler CLI
- `/app/bin/hlx_lsp` - HLX Language Server

### Environment

- **User:** `hlx` (UID 1000, non-root)
- **Working Directory:** `/home/hlx`
- **Examples:** `/app/examples`
- **PATH:** `/app/bin:$PATH`

---

## Usage Examples

### Compile HLX Files

```bash
# Mount your source directory
docker run --rm -v $(pwd):/workspace hlx-compiler:latest \
  hlx compile /workspace/my_program.hlx -o /workspace/output.bin

# Using docker-compose:
docker-compose run --rm -v $(pwd):/workspace hlx-compiler \
  hlx compile /workspace/my_program.hlx
```

### Run HLX Programs

```bash
# Execute a compiled HLX program
docker run --rm -v $(pwd):/workspace hlx-compiler:latest \
  hlx run /workspace/my_program.hlx

# With backend selection:
docker-compose run --rm hlx-compiler \
  hlx run --backend cpu examples/tensor_ops.hlx
```

### Language Server Integration

#### Stdio Mode (Default)

Most editors use stdio for LSP communication:

```bash
# VS Code example (in settings.json):
{
  "hlx.lsp.command": "docker",
  "hlx.lsp.args": [
    "run", "--rm", "-i",
    "-v", "${workspaceFolder}:/workspace",
    "hlx-compiler:latest",
    "hlx_lsp"
  ]
}
```

#### TCP Mode

For network-based LSP communication:

```bash
# Start LSP server on port 9257
docker-compose up hlx-lsp

# Configure editor to connect to localhost:9257
```

---

## Docker Compose Services

### `hlx-compiler`

Runs the HLX compiler for one-off compilation tasks.

```bash
docker-compose run --rm hlx-compiler hlx compile examples/tensor.hlx
```

**Volumes:**
- `./examples:/workspace/examples:ro` (read-only examples)
- `./output:/workspace/output` (compilation output)

### `hlx-lsp`

Runs the HLX Language Server for editor integration.

```bash
docker-compose up hlx-lsp
```

**Ports:**
- `9257:9257` (LSP TCP port)

**Volumes:**
- `./examples:/workspace:ro` (workspace files)
- `hlx-lsp-cache:/home/hlx/.hlx` (persistent cache)

### `hlx-dev`

Interactive development environment with full HLX toolchain.

```bash
docker-compose run --rm hlx-dev bash
```

**Volumes:**
- `.:/workspace` (entire project mounted)
- Persistent caches for LSP and Cargo

---

## Advanced Configuration

### Custom Build Arguments

```bash
# Build with specific LLVM version
docker build \
  --build-arg LLVM_VERSION=18 \
  -t hlx-compiler:llvm18 .
```

### Volume Mounts

```bash
# Mount custom source directory
docker run --rm \
  -v /path/to/hlx/project:/workspace \
  -v /path/to/output:/output \
  hlx-compiler:latest \
  hlx compile /workspace/main.hlx -o /output/main.bin
```

### Environment Variables

```bash
# Set HLX runtime options
docker run --rm \
  -e HLX_BACKEND=cpu \
  -e HLX_LOG_LEVEL=debug \
  -v $(pwd):/workspace \
  hlx-compiler:latest \
  hlx run /workspace/program.hlx
```

---

## Performance Considerations

### Build Cache

Docker caches build layers. To force a fresh build:

```bash
docker-compose build --no-cache
```

### Cargo Cache

Mount Cargo cache to speed up rebuilds:

```bash
docker run --rm \
  -v cargo-cache:/usr/local/cargo/registry \
  -v $(pwd):/workspace \
  rust:1.75 \
  cargo build --release
```

### Compilation Performance

The release build uses LTO and single codegen-unit for optimal performance:

```toml
[profile.release]
lto = true
codegen-units = 1
panic = "abort"
```

This increases build time (~10-15 min) but produces highly optimized binaries.

---

## CI/CD Integration

### GitHub Actions

```yaml
name: Build HLX Docker

on: [push, pull_request]

jobs:
  docker:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Build Docker image
        run: docker build -t hlx-compiler:${{ github.sha }} .

      - name: Run tests
        run: docker run --rm hlx-compiler:${{ github.sha }} hlx test
```

### GitLab CI

```yaml
build:
  image: docker:latest
  services:
    - docker:dind
  script:
    - docker build -t hlx-compiler:latest .
    - docker run --rm hlx-compiler:latest hlx --version
```

---

## Troubleshooting

### LLVM Errors

If you see LLVM-related errors:

```bash
# Ensure LLVM 18 is installed in builder
docker build --progress=plain . 2>&1 | grep LLVM
```

### Permission Errors

If files created by Docker are owned by root:

```bash
# Run with your user ID
docker run --rm --user $(id -u):$(id -g) \
  -v $(pwd):/workspace \
  hlx-compiler:latest \
  hlx compile /workspace/file.hlx
```

### Large Image Size

If the image is too large:

```bash
# Check layer sizes
docker history hlx-compiler:latest

# Clean up build artifacts
docker builder prune
```

### LSP Connection Issues

If the LSP server won't connect:

```bash
# Check if container is running
docker ps | grep hlx-lsp

# View logs
docker logs hlx-lsp

# Test stdio mode
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | \
  docker run --rm -i hlx-compiler:latest hlx_lsp
```

---

## Security Notes

### Non-Root User

The runtime image uses a non-root user (`hlx`) for security:

```dockerfile
RUN useradd -m -u 1000 -s /bin/bash hlx
USER hlx
```

### Minimal Runtime Dependencies

Only essential runtime libraries are included:

- `ca-certificates` - SSL/TLS certificates
- `libssl3` - OpenSSL runtime
- `libsdl2-2.0-0` - SDL2 runtime
- `libvulkan1` - Vulkan runtime
- `llvm-18-runtime` - LLVM runtime

### Network Isolation

By default, containers run in isolated networks. For production:

```yaml
services:
  hlx-lsp:
    networks:
      - internal
    # No ports exposed to host
```

---

## Development Workflow

### Live Reloading

For development with live reloading:

```bash
# Mount source as volume
docker-compose run --rm \
  -v $(pwd):/workspace \
  hlx-dev bash

# Inside container:
cd /workspace
cargo watch -x 'build --bin hlx'
```

### Debugging

```bash
# Run with debug symbols
docker run --rm -it \
  --cap-add=SYS_PTRACE \
  -v $(pwd):/workspace \
  hlx-compiler:latest \
  bash -c "cd /workspace && gdb hlx"
```

### Testing

```bash
# Run all tests inside container
docker-compose run --rm hlx-dev \
  cargo test --all

# Run specific test
docker-compose run --rm hlx-dev \
  cargo test --bin hlx -- test_compile
```

---

## Production Deployment

### Registry Push

```bash
# Tag for registry
docker tag hlx-compiler:latest registry.example.com/hlx-compiler:0.1.0

# Push to registry
docker push registry.example.com/hlx-compiler:0.1.0
```

### Kubernetes

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: hlx-lsp
spec:
  containers:
  - name: hlx-lsp
    image: registry.example.com/hlx-compiler:0.1.0
    command: ["hlx_lsp"]
    ports:
    - containerPort: 9257
    volumeMounts:
    - name: workspace
      mountPath: /workspace
  volumes:
  - name: workspace
    persistentVolumeClaim:
      claimName: hlx-workspace
```

### Docker Swarm

```bash
# Deploy as service
docker service create \
  --name hlx-lsp \
  --publish 9257:9257 \
  --mount type=volume,source=hlx-data,target=/home/hlx/.hlx \
  hlx-compiler:latest \
  hlx_lsp
```

---

## Additional Resources

- **HLX Documentation:** `/app/examples/`
- **Dockerfile:** `Dockerfile`
- **Compose Config:** `docker-compose.yml`
- **Issue Tracker:** https://github.com/latentcollapse/hlx-compiler/issues

---

## Version History

- **0.1.0** (2026-01-16)
  - Initial Docker support
  - Multi-stage build with LLVM 18
  - Docker Compose with compiler, LSP, and dev services
  - Security hardening (non-root user)

---

*For more information, see the main [README.md](README.md)*

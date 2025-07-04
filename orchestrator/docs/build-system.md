# Build System Documentation

## Overview

The orchestrator platform uses a comprehensive build system that creates multiple binaries and Docker images for different components. This document explains how to build, test, and deploy the platform.

## Components

### Binaries
- **orchestrator** - Main Kubernetes controller and API server
- **toolman** - MCP server proxy with tool filtering and access control
- **mcp-wrapper** - Lightweight MCP forwarder for agent containers

### Docker Images
- **orchestrator** - Main controller image for Kubernetes deployment
- **toolman** - Toolman server image for sidecar deployment
- **mcp-wrapper** - Lightweight wrapper image (standalone usage)
- **claude-code** - Claude Code with integrated MCP wrapper (optional)

## Quick Start

### Build Everything
```bash
# Build all components with default settings
./scripts/build-all.sh

# Build with specific tag and push to registry
./scripts/build-all.sh -t v1.0.0 -r ghcr.io/myorg -p

# Build including Claude Code image
./scripts/build-all.sh --claude-image
```

### Build Individual Components
```bash
# Build just the toolman components
./scripts/build-toolman.sh --docker --tag v1.0.0

# Build main orchestrator
./scripts/build.sh -t v1.0.0
```

## Build Script Options

### scripts/build-all.sh
Comprehensive build script for the entire platform.

```bash
Usage: ./scripts/build-all.sh [OPTIONS]

OPTIONS:
    -h, --help          Show help message
    -t, --tag TAG       Set image tag (default: latest)
    -r, --registry REG  Set registry prefix (default: ghcr.io/5dlabs/platform)
    -p, --push          Push images to registry after building
    --debug             Build in debug mode (default: release)
    --no-docker         Skip Docker image building
    --skip-tests        Skip running tests before building
    --claude-image      Build Claude Code Docker image with MCP wrapper
    --no-cache          Build Docker images without cache
```

### Examples

#### Development Build
```bash
# Quick development build - debug mode, no Docker
./scripts/build-all.sh --debug --no-docker

# Test build with Docker but no push
./scripts/build-all.sh -t test-build
```

#### Production Build
```bash
# Full production build and push
./scripts/build-all.sh -t v1.2.0 -r ghcr.io/mycompany -p

# Production build with Claude Code integration
./scripts/build-all.sh -t v1.2.0 -r ghcr.io/mycompany -p --claude-image
```

#### CI/CD Pipeline
```bash
# Automated build for CI/CD (skip tests that may have run separately)
./scripts/build-all.sh --skip-tests -t ${VERSION} -r ${REGISTRY} -p
```

## Docker Images

### orchestrator
**Purpose**: Main Kubernetes controller
**Base**: debian:bookworm-slim
**Size**: ~50MB
**Usage**:
```bash
docker run ghcr.io/5dlabs/platform/orchestrator:latest
```

### toolman
**Purpose**: MCP server proxy with tool filtering
**Base**: debian:bookworm-slim  
**Size**: ~40MB
**Ports**: 3000 (HTTP API)
**Usage**:
```bash
docker run -p 3000:3000 ghcr.io/5dlabs/platform/toolman:latest
```

### mcp-wrapper
**Purpose**: Lightweight MCP forwarder for agents
**Base**: debian:bookworm-slim
**Size**: ~30MB
**Usage**:
```bash
# Direct mode (stdin/stdout proxy)
echo '{"jsonrpc":"2.0","method":"initialize"}' | docker run -i ghcr.io/5dlabs/platform/mcp-wrapper:latest

# With environment configuration
docker run -e MCP_TOOLMAN_SERVER_URL=http://toolman:3000/mcp ghcr.io/5dlabs/platform/mcp-wrapper:latest
```

### claude-code (Optional)
**Purpose**: Claude Code with integrated MCP wrapper
**Base**: python:3.11-slim
**Size**: ~200MB
**Ports**: 8080 (health check)
**Usage**:
```bash
# Direct mode
docker run ghcr.io/5dlabs/platform/claude-code:latest direct --task "Fix the bug"

# Subprocess mode
docker run ghcr.io/5dlabs/platform/claude-code:latest subprocess --api-key=$CLAUDE_API_KEY

# Interactive shell
docker run -it ghcr.io/5dlabs/platform/claude-code:latest shell
```

## Multi-Container Architecture

### Typical Deployment
```yaml
# Agent Pod with Toolman Sidecar
apiVersion: v1
kind: Pod
spec:
  containers:
  - name: claude-agent
    image: ghcr.io/5dlabs/platform/claude-code:latest
    env:
    - name: MCP_TOOLMAN_SERVER_URL
      value: "http://localhost:3000/mcp"
  - name: toolman
    image: ghcr.io/5dlabs/platform/toolman:latest
    ports:
    - containerPort: 3000
```

## Build Process Details

### Stage 1: Code Quality Checks
- `cargo test --verbose` - Run all unit and integration tests
- `cargo clippy --all-targets --all-features -- -D warnings` - Lint checks
- `cargo fmt --all -- --check` - Code formatting verification

### Stage 2: Binary Compilation
- Builds all three binaries in release mode (or debug if specified)
- Verifies binary creation and reports sizes
- Uses Rust's optimized release profile for production builds

### Stage 3: Docker Image Building
- Multi-stage builds for minimal image sizes
- Copies only necessary binaries and runtime dependencies
- Security-focused: non-root users, minimal attack surface
- Health checks for production readiness

### Stage 4: Registry Push (Optional)
- Pushes images to specified registry
- Supports multi-platform builds with Docker Buildx
- Verifies successful push operations

## Configuration Files

### Cargo Configuration
- `Cargo.toml` - Main workspace configuration
- `orchestrator-core/Cargo.toml` - Core library with all binaries
- `orchestrator-common/Cargo.toml` - Shared types and utilities

### Docker Configuration
- `Dockerfile` - Main orchestrator image
- `Dockerfile.toolman` - Toolman server image
- `Dockerfile.mcp-wrapper` - MCP wrapper image  
- `Dockerfile.claude-code` - Claude Code with MCP integration
- `.dockerignore` - Files to exclude from Docker context

### Build Scripts
- `scripts/build-all.sh` - Comprehensive build script
- `scripts/build-toolman.sh` - Toolman-specific builds
- `scripts/build.sh` - Main orchestrator builds
- `scripts/ci-checks.sh` - CI/CD validation script

## Environment Variables

### Build Time
- `CARGO_TARGET_DIR` - Override target directory
- `RUST_LOG` - Logging level for build process

### Runtime (Docker)
- `MCP_TOOLMAN_SERVER_URL` - Toolman server endpoint
- `MCP_WRAPPER_DEBUG` - Enable debug logging
- `MCP_WRAPPER_TIMEOUT` - Request timeout seconds
- `CLAUDE_API_KEY` - Claude API authentication
- `RUST_LOG` - Runtime logging level

## Troubleshooting

### Common Build Issues

#### OpenSSL Dependencies
```bash
# On Ubuntu/Debian
sudo apt-get install libssl-dev pkg-config

# On macOS
brew install openssl pkg-config
```

#### Docker Buildx Issues
```bash
# Enable buildx
docker buildx create --use

# For multi-platform builds
docker buildx build --platform linux/amd64,linux/arm64 --push -t myimage .
```

#### Permission Issues
```bash
# Make scripts executable
chmod +x scripts/*.sh docker/*.sh
```

### Debug Build Issues
```bash
# Build with verbose output
./scripts/build-all.sh --debug --skip-tests

# Check individual components
cargo build --bin toolman --verbose
cargo build --bin mcp-wrapper --verbose
```

### Docker Issues
```bash
# Test individual Dockerfiles
docker build -f Dockerfile.toolman -t test-toolman .
docker run --rm test-toolman --help

# Check image contents
docker run -it --entrypoint /bin/bash test-toolman
```

## Performance Considerations

### Build Optimization
- Use `--release` builds for production
- Enable LTO and codegen-units optimization in Cargo.toml
- Use multi-stage Docker builds to minimize image size
- Cache Cargo dependencies in Docker builds

### Runtime Optimization
- toolman: ~10MB memory usage, <100ms response time
- mcp-wrapper: ~5MB memory usage, <50ms forwarding latency
- Statically linked binaries for minimal dependencies

## CI/CD Integration

### GitHub Actions Example
```yaml
name: Build and Push
on:
  push:
    tags: ['v*']

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - name: Build and push
      run: |
        ./scripts/build-all.sh \
          -t ${GITHUB_REF#refs/tags/} \
          -r ghcr.io/${{ github.repository_owner }} \
          -p
```

### Local Development
```bash
# Quick iteration cycle
./scripts/build-all.sh --debug --no-docker
cargo test
./target/debug/toolman &
./target/debug/mcp-wrapper
```

## Security Considerations

### Image Security
- Non-root users in all containers
- Minimal base images (debian:bookworm-slim)
- No package managers in runtime images
- Health checks for container monitoring

### Binary Security
- Static linking reduces attack surface
- No debug symbols in release builds
- Rust's memory safety guarantees
- Minimal external dependencies

### Network Security
- HTTP-only internal communication
- Configurable timeouts
- No privileged ports required
- Container-to-container communication over localhost
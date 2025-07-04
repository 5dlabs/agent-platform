#!/bin/bash
# Comprehensive build script for the entire orchestrator platform
# Builds all binaries, Docker images, and prepares for deployment

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default configuration
TAG="latest"
REGISTRY="ghcr.io/5dlabs/platform"
BUILD_TYPE="release"
DOCKER_BUILD=true
PUSH=false
SKIP_TESTS=false
CLAUDE_IMAGE=false

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_header() {
    echo -e "${BLUE}[BUILD]${NC} $1"
}

# Function to show usage
usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Build the complete orchestrator platform including all binaries and Docker images.

OPTIONS:
    -h, --help          Show this help message
    -t, --tag TAG       Set the image tag (default: latest)
    -r, --registry REG  Set the registry prefix (default: ghcr.io/5dlabs/platform)
    -p, --push          Push images to registry after building
    --debug             Build in debug mode (default: release)
    --no-docker         Skip Docker image building
    --skip-tests        Skip running tests before building
    --claude-image      Build Claude Code Docker image with MCP wrapper
    --no-cache          Build Docker images without cache

EXAMPLES:
    $0                                      # Build everything locally
    $0 -t v1.0.0 -p                       # Build and push with version tag
    $0 --claude-image                      # Include Claude Code Docker image
    $0 --debug --no-docker                 # Debug build, binaries only

Components built:
    - orchestrator (main controller)
    - toolman (MCP server proxy)
    - mcp-wrapper (lightweight MCP forwarder)
    - Docker images for each component
    - Claude Code image (if --claude-image specified)
EOF
}

# Parse command line arguments
DOCKER_ARGS=""
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            usage
            exit 0
            ;;
        -t|--tag)
            TAG="$2"
            shift 2
            ;;
        -r|--registry)
            REGISTRY="$2"
            shift 2
            ;;
        -p|--push)
            PUSH=true
            shift
            ;;
        --debug)
            BUILD_TYPE="debug"
            shift
            ;;
        --no-docker)
            DOCKER_BUILD=false
            shift
            ;;
        --skip-tests)
            SKIP_TESTS=true
            shift
            ;;
        --claude-image)
            CLAUDE_IMAGE=true
            shift
            ;;
        --no-cache)
            DOCKER_ARGS="$DOCKER_ARGS --no-cache"
            shift
            ;;
        *)
            print_error "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

# Get script directory and change to project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_DIR"

# Verify we're in the right directory
if [[ ! -f "Cargo.toml" ]]; then
    print_error "Cargo.toml not found. Run this script from the orchestrator directory."
    exit 1
fi

print_header "🚀 Building Orchestrator Platform"
print_status "Configuration:"
print_status "  Build type: $BUILD_TYPE"
print_status "  Tag: $TAG"
print_status "  Registry: $REGISTRY"
print_status "  Docker build: $DOCKER_BUILD"
print_status "  Push: $PUSH"
print_status "  Claude image: $CLAUDE_IMAGE"

# Step 1: Run tests and checks (unless skipped)
if [[ "$SKIP_TESTS" != true ]]; then
    print_header "🧪 Running Tests and Checks"
    
    print_status "Running cargo tests..."
    cargo test --verbose || {
        print_error "Tests failed!"
        exit 1
    }

    print_status "Running clippy checks..."
    cargo clippy --all-targets --all-features -- -D warnings || {
        print_error "Clippy checks failed!"
        exit 1
    }

    print_status "Checking code formatting..."
    cargo fmt --all -- --check || {
        print_error "Code formatting check failed! Run 'cargo fmt' to fix."
        exit 1
    }
    
    print_status "✅ All tests and checks passed!"
fi

# Step 2: Build binaries
print_header "🔧 Building Binaries"

if [[ "$BUILD_TYPE" == "release" ]]; then
    print_status "Building release binaries..."
    cargo build --release --bin orchestrator
    cargo build --release --bin toolman
    cargo build --release --bin mcp-wrapper
    BINARY_DIR="target/release"
else
    print_status "Building debug binaries..."
    cargo build --bin orchestrator
    cargo build --bin toolman
    cargo build --bin mcp-wrapper
    BINARY_DIR="target/debug"
fi

# Verify binaries were built
for binary in orchestrator toolman mcp-wrapper; do
    if [[ ! -f "$BINARY_DIR/$binary" ]]; then
        print_error "$binary binary not found at $BINARY_DIR/$binary"
        exit 1
    fi
done

print_status "✅ All binaries built successfully!"
print_status "Binary sizes:"
ls -lh "$BINARY_DIR"/{orchestrator,toolman,mcp-wrapper} | awk '{print "  " $9 ": " $5}'

# Step 3: Build Docker images
if [[ "$DOCKER_BUILD" == true ]]; then
    print_header "🐳 Building Docker Images"
    
    # Array of images to build: [dockerfile:image_name:description]
    IMAGES=(
        "Dockerfile:orchestrator:Main orchestrator controller"
        "Dockerfile.toolman:toolman:MCP server proxy with tool filtering"
        "Dockerfile.mcp-wrapper:mcp-wrapper:Lightweight MCP forwarder for agents"
    )
    
    # Add Claude Code image if requested
    if [[ "$CLAUDE_IMAGE" == true ]]; then
        IMAGES+=("Dockerfile.claude-code:claude-code:Claude Code with MCP wrapper integration")
    fi
    
    for image_spec in "${IMAGES[@]}"; do
        IFS=':' read -r dockerfile image_name description <<< "$image_spec"
        full_image_name="$REGISTRY/$image_name:$TAG"
        
        print_status "Building $description..."
        print_status "  Dockerfile: $dockerfile"
        print_status "  Image: $full_image_name"
        
        if [[ -f "$dockerfile" ]]; then
            docker build $DOCKER_ARGS -f "$dockerfile" -t "$full_image_name" . || {
                print_error "Failed to build $image_name"
                exit 1
            }
        else
            print_warning "Dockerfile $dockerfile not found, skipping $image_name"
        fi
    done
    
    print_status "✅ All Docker images built successfully!"
    
    # Show image sizes
    print_status "Image sizes:"
    for image_spec in "${IMAGES[@]}"; do
        IFS=':' read -r dockerfile image_name description <<< "$image_spec"
        full_image_name="$REGISTRY/$image_name:$TAG"
        if docker image inspect "$full_image_name" >/dev/null 2>&1; then
            size=$(docker image inspect "$full_image_name" --format='{{.Size}}' | numfmt --to=iec)
            echo "  $image_name: $size"
        fi
    done
fi

# Step 4: Push images (if requested)
if [[ "$PUSH" == true ]] && [[ "$DOCKER_BUILD" == true ]]; then
    print_header "📦 Pushing Images to Registry"
    
    for image_spec in "${IMAGES[@]}"; do
        IFS=':' read -r dockerfile image_name description <<< "$image_spec"
        full_image_name="$REGISTRY/$image_name:$TAG"
        
        if docker image inspect "$full_image_name" >/dev/null 2>&1; then
            print_status "Pushing $image_name..."
            docker push "$full_image_name" || {
                print_error "Failed to push $image_name"
                exit 1
            }
        fi
    done
    
    print_status "✅ All images pushed successfully!"
fi

# Step 5: Create build summary
print_header "📋 Build Summary"

echo ""
print_status "🎯 Binaries built:"
for binary in orchestrator toolman mcp-wrapper; do
    if [[ -f "$BINARY_DIR/$binary" ]]; then
        size=$(ls -lh "$BINARY_DIR/$binary" | awk '{print $5}')
        echo "  ✅ $binary ($size)"
    else
        echo "  ❌ $binary (missing)"
    fi
done

if [[ "$DOCKER_BUILD" == true ]]; then
    echo ""
    print_status "🐳 Docker images:"
    for image_spec in "${IMAGES[@]}"; do
        IFS=':' read -r dockerfile image_name description <<< "$image_spec"
        full_image_name="$REGISTRY/$image_name:$TAG"
        if docker image inspect "$full_image_name" >/dev/null 2>&1; then
            echo "  ✅ $full_image_name"
        else
            echo "  ❌ $full_image_name (not built)"
        fi
    done
fi

echo ""
print_status "🚀 Platform ready for deployment!"

# Usage instructions
print_header "📖 Usage Instructions"
echo ""
echo "Local testing:"
echo "  ./$BINARY_DIR/orchestrator"
echo "  ./$BINARY_DIR/toolman"
echo "  ./$BINARY_DIR/mcp-wrapper"
echo ""

if [[ "$DOCKER_BUILD" == true ]]; then
    echo "Docker deployment:"
    echo "  docker run $REGISTRY/orchestrator:$TAG"
    echo "  docker run $REGISTRY/toolman:$TAG"
    echo "  docker run $REGISTRY/mcp-wrapper:$TAG"
    if [[ "$CLAUDE_IMAGE" == true ]]; then
        echo "  docker run $REGISTRY/claude-code:$TAG"
    fi
    echo ""
fi

if [[ "$PUSH" == true ]]; then
    echo "Images are available at:"
    echo "  $REGISTRY/orchestrator:$TAG"
    echo "  $REGISTRY/toolman:$TAG"  
    echo "  $REGISTRY/mcp-wrapper:$TAG"
    if [[ "$CLAUDE_IMAGE" == true ]]; then
        echo "  $REGISTRY/claude-code:$TAG"
    fi
fi

print_status "✅ Build completed successfully!"
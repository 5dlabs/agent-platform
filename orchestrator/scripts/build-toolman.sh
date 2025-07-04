#!/bin/bash
# Build script for Toolman server and MCP wrapper

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Building Toolman server and MCP wrapper...${NC}"

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Change to project directory
cd "$PROJECT_DIR"

# Check if we're in the right directory
if [[ ! -f "Cargo.toml" ]]; then
    echo -e "${RED}Error: Cargo.toml not found. Run this script from the orchestrator directory.${NC}"
    exit 1
fi

# Parse command line arguments
BUILD_TYPE="release"
DOCKER_BUILD=false
DOCKER_TAG="latest"
DOCKER_REGISTRY="ghcr.io/5dlabs/platform"

while [[ $# -gt 0 ]]; do
    case $1 in
        --debug)
            BUILD_TYPE="debug"
            shift
            ;;
        --docker)
            DOCKER_BUILD=true
            shift
            ;;
        --tag)
            DOCKER_TAG="$2"
            shift 2
            ;;
        --registry)
            DOCKER_REGISTRY="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo "Options:"
            echo "  --debug          Build in debug mode (default: release)"
            echo "  --docker         Build Docker images"
            echo "  --tag TAG        Docker tag (default: latest)"
            echo "  --registry REG   Docker registry (default: ghcr.io/5dlabs/platform)"
            echo "  --help           Show this help message"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

echo -e "${YELLOW}Building binaries in $BUILD_TYPE mode...${NC}"

# Build the binaries
if [[ "$BUILD_TYPE" == "release" ]]; then
    echo "Building toolman binary..."
    cargo build --release --bin toolman
    
    echo "Building mcp-wrapper binary..."
    cargo build --release --bin mcp-wrapper
    
    BINARY_DIR="target/release"
else
    echo "Building toolman binary (debug)..."
    cargo build --bin toolman
    
    echo "Building mcp-wrapper binary (debug)..."
    cargo build --bin mcp-wrapper
    
    BINARY_DIR="target/debug"
fi

# Check if binaries were built successfully
if [[ ! -f "$BINARY_DIR/toolman" ]]; then
    echo -e "${RED}Error: toolman binary not found at $BINARY_DIR/toolman${NC}"
    exit 1
fi

if [[ ! -f "$BINARY_DIR/mcp-wrapper" ]]; then
    echo -e "${RED}Error: mcp-wrapper binary not found at $BINARY_DIR/mcp-wrapper${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Binaries built successfully:${NC}"
echo "  toolman: $BINARY_DIR/toolman"
echo "  mcp-wrapper: $BINARY_DIR/mcp-wrapper"

# Show binary sizes
echo -e "${YELLOW}Binary sizes:${NC}"
ls -lh "$BINARY_DIR/toolman" "$BINARY_DIR/mcp-wrapper" | awk '{print "  " $9 ": " $5}'

# Build Docker images if requested
if [[ "$DOCKER_BUILD" == "true" ]]; then
    echo -e "${YELLOW}Building Docker images...${NC}"
    
    # Build toolman image
    echo "Building toolman Docker image..."
    docker build -f Dockerfile.toolman -t "$DOCKER_REGISTRY/toolman:$DOCKER_TAG" .
    
    # Build mcp-wrapper image
    echo "Building mcp-wrapper Docker image..."
    docker build -f Dockerfile.mcp-wrapper -t "$DOCKER_REGISTRY/mcp-wrapper:$DOCKER_TAG" .
    
    echo -e "${GREEN}✅ Docker images built successfully:${NC}"
    echo "  $DOCKER_REGISTRY/toolman:$DOCKER_TAG"
    echo "  $DOCKER_REGISTRY/mcp-wrapper:$DOCKER_TAG"
    
    # Show image sizes
    echo -e "${YELLOW}Image sizes:${NC}"
    docker images "$DOCKER_REGISTRY/toolman:$DOCKER_TAG" "$DOCKER_REGISTRY/mcp-wrapper:$DOCKER_TAG" --format "table {{.Repository}}:{{.Tag}}\t{{.Size}}"
fi

echo -e "${GREEN}✅ Build complete!${NC}"

# Show usage instructions
echo -e "${YELLOW}Usage instructions:${NC}"
echo "  Local testing:"
echo "    ./$BINARY_DIR/toolman"
echo "    ./$BINARY_DIR/mcp-wrapper"
echo ""
if [[ "$DOCKER_BUILD" == "true" ]]; then
    echo "  Docker deployment:"
    echo "    docker run $DOCKER_REGISTRY/toolman:$DOCKER_TAG"
    echo "    docker run $DOCKER_REGISTRY/mcp-wrapper:$DOCKER_TAG"
    echo ""
    echo "  Push to registry:"
    echo "    docker push $DOCKER_REGISTRY/toolman:$DOCKER_TAG"
    echo "    docker push $DOCKER_REGISTRY/mcp-wrapper:$DOCKER_TAG"
fi
#!/bin/bash
# Entrypoint script for Claude Code with MCP wrapper integration
# Handles different execution modes and ensures proper MCP integration

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
log_info() {
    echo -e "${GREEN}[Claude]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[Claude]${NC} $1"
}

log_error() {
    echo -e "${RED}[Claude]${NC} $1"
}

log_debug() {
    if [[ "${MCP_WRAPPER_DEBUG:-false}" == "true" ]]; then
        echo -e "${BLUE}[Claude DEBUG]${NC} $1"
    fi
}

# Default configuration
TOOLMAN_URL="${MCP_TOOLMAN_SERVER_URL:-http://toolman:3000/mcp}"
WRAPPER_DEBUG="${MCP_WRAPPER_DEBUG:-false}"
WRAPPER_TIMEOUT="${MCP_WRAPPER_TIMEOUT:-30}"

# Function to check if toolman is available
check_toolman() {
    local max_attempts=30
    local attempt=1
    
    log_info "Checking toolman connectivity at $TOOLMAN_URL..."
    
    while [ $attempt -le $max_attempts ]; do
        if curl -s -f "$TOOLMAN_URL" >/dev/null 2>&1 || \
           curl -s -f "${TOOLMAN_URL%/mcp}/health" >/dev/null 2>&1; then
            log_info "✅ Toolman is accessible"
            return 0
        fi
        
        log_warn "Attempt $attempt/$max_attempts: Waiting for toolman..."
        sleep 2
        ((attempt++))
    done
    
    log_error "❌ Could not connect to toolman after $max_attempts attempts"
    log_error "   URL: $TOOLMAN_URL"
    log_warn "   Continuing without toolman (MCP tools will not be available)"
    return 1
}

# Function to test MCP wrapper
test_mcp_wrapper() {
    log_info "Testing MCP wrapper..."
    
    if command -v mcp-wrapper >/dev/null 2>&1; then
        # Test with a simple initialization message
        local test_message='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}'
        
        if echo "$test_message" | timeout 5 mcp-wrapper >/dev/null 2>&1; then
            log_info "✅ MCP wrapper is functional"
            return 0
        else
            log_warn "⚠️  MCP wrapper test failed"
            return 1
        fi
    else
        log_error "❌ MCP wrapper binary not found"
        return 1
    fi
}

# Function to show usage
show_usage() {
    cat << EOF
Claude Code with MCP Wrapper Integration

Usage:
  $(basename "$0") [MODE] [OPTIONS]

Modes:
  direct              Run Claude Code directly (no subprocess mode)
  subprocess          Run Claude Code as subprocess with MCP wrapper proxy
  shell              Open interactive shell with Claude and MCP tools available
  --help             Show this help message

Examples:
  # Direct mode (default)
  $(basename "$0") direct --task "Fix the authentication bug"
  
  # Subprocess mode with MCP proxy
  $(basename "$0") subprocess --api-key=\$CLAUDE_API_KEY
  
  # Interactive shell
  $(basename "$0") shell

Environment Variables:
  MCP_TOOLMAN_SERVER_URL     Toolman server URL (default: http://toolman:3000/mcp)
  MCP_WRAPPER_DEBUG          Enable debug logging (default: false)
  MCP_WRAPPER_TIMEOUT        Request timeout in seconds (default: 30)
  CLAUDE_API_KEY             Claude API key for authentication
  RUST_LOG                   Rust logging level (default: info)

EOF
}

# Function to run Claude in direct mode
run_direct_mode() {
    log_info "Starting Claude Code in direct mode with MCP wrapper"
    
    # Set up MCP wrapper as the MCP server for Claude
    export MCP_SERVER_COMMAND="mcp-wrapper"
    
    # Run Claude Code with remaining arguments
    exec claude-code "$@"
}

# Function to run Claude in subprocess mode
run_subprocess_mode() {
    log_info "Starting Claude Code in subprocess mode with MCP wrapper proxy"
    
    # Start MCP wrapper with Claude as subprocess
    # The wrapper will launch Claude and proxy MCP communication
    exec mcp-wrapper claude-code "$@"
}

# Function to start interactive shell
run_shell_mode() {
    log_info "Starting interactive shell with Claude Code and MCP tools"
    
    # Show available tools
    echo ""
    log_info "Available tools:"
    echo "  claude-code      - Claude Code CLI"
    echo "  mcp-wrapper      - MCP wrapper for toolman integration"
    echo ""
    log_info "Environment:"
    echo "  Toolman URL: $TOOLMAN_URL"
    echo "  MCP Debug: $WRAPPER_DEBUG"
    echo "  Workspace: $(pwd)"
    echo ""
    log_info "Try: claude-code --help"
    echo ""
    
    # Start interactive bash shell
    exec /bin/bash
}

# Main execution logic
main() {
    log_info "🚀 Claude Code with MCP Wrapper starting..."
    
    # Show configuration
    log_debug "Configuration:"
    log_debug "  Toolman URL: $TOOLMAN_URL"
    log_debug "  Debug mode: $WRAPPER_DEBUG"
    log_debug "  Timeout: ${WRAPPER_TIMEOUT}s"
    log_debug "  Working directory: $(pwd)"
    
    # Check if we have any arguments
    if [[ $# -eq 0 ]]; then
        show_usage
        exit 0
    fi
    
    # Handle help
    if [[ "$1" == "--help" ]] || [[ "$1" == "-h" ]]; then
        show_usage
        exit 0
    fi
    
    # Check toolman connectivity (non-blocking)
    check_toolman || true
    
    # Test MCP wrapper
    test_mcp_wrapper || log_warn "MCP wrapper may not function correctly"
    
    # Determine execution mode
    case "$1" in
        direct)
            shift
            run_direct_mode "$@"
            ;;
        subprocess)
            shift
            run_subprocess_mode "$@"
            ;;
        shell)
            run_shell_mode
            ;;
        *)
            # Default to direct mode if no mode specified
            log_info "No mode specified, defaulting to direct mode"
            run_direct_mode "$@"
            ;;
    esac
}

# Execute main function with all arguments
main "$@"
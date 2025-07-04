#!/bin/bash
# Entrypoint script for Claude agent container
# Decides whether to use MCP wrapper or run Claude directly

set -e

# Check if toolman/MCP wrapper is enabled
if [[ "${MCP_WRAPPER_ENABLED}" == "true" ]]; then
    echo "🔧 MCP Wrapper enabled - starting Claude through toolman proxy"
    
    # Verify toolman server is available
    if [[ -n "${MCP_TOOLMAN_SERVER_URL}" ]]; then
        echo "🌐 Toolman server: ${MCP_TOOLMAN_SERVER_URL}"
        
        # Wait for toolman to be ready
        echo "⏳ Waiting for toolman server to be ready..."
        timeout=30
        while ! curl -f "${MCP_TOOLMAN_SERVER_URL%/mcp}/health" >/dev/null 2>&1; do
            timeout=$((timeout - 1))
            if [[ $timeout -le 0 ]]; then
                echo "❌ Timeout waiting for toolman server"
                exit 1
            fi
            sleep 1
        done
        echo "✅ Toolman server is ready"
    fi
    
    # Launch Claude through MCP wrapper
    # The wrapper will:
    # 1. Start Claude as a child process
    # 2. Proxy MCP communication between Claude and toolman
    # 3. Forward stdin/stdout for non-MCP communication
    exec mcp-wrapper claude "$@"
    
else
    echo "🚀 Starting Claude directly (no MCP wrapper)"
    # Run Claude directly without MCP proxying
    exec claude "$@"
fi
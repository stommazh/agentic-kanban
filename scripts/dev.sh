#!/bin/bash
# Start development servers for vibe-kanban
# Usage: ./scripts/dev.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

BACKEND_PORT=${BACKEND_PORT:-3001}
FRONTEND_PORT=${FRONTEND_PORT:-3000}

echo "🚀 Starting vibe-kanban development servers..."
echo "   Backend:  http://localhost:$BACKEND_PORT"
echo "   Frontend: http://localhost:$FRONTEND_PORT"
echo ""

# Check if release binary exists
if [ ! -f "./target/release/server" ]; then
    echo "📦 Building release binary..."
    cargo build --release --bin server
fi

# Function to cleanup on exit
cleanup() {
    echo ""
    echo "🛑 Shutting down..."
    kill $BACKEND_PID 2>/dev/null || true
    kill $FRONTEND_PID 2>/dev/null || true
    exit 0
}
trap cleanup SIGINT SIGTERM

# Start backend
echo "🔧 Starting backend on port $BACKEND_PORT..."
PORT=$BACKEND_PORT RUST_LOG=info ./target/release/server &
BACKEND_PID=$!

# Wait for backend to start
sleep 2

# Check if backend is running
if ! kill -0 $BACKEND_PID 2>/dev/null; then
    echo "❌ Backend failed to start"
    exit 1
fi

# Start frontend
echo "🎨 Starting frontend on port $FRONTEND_PORT..."
cd frontend
BACKEND_PORT=$BACKEND_PORT npm run dev -- --port $FRONTEND_PORT --host &
FRONTEND_PID=$!
cd ..

echo ""
echo "✅ Development servers running!"
echo "   Frontend: http://localhost:$FRONTEND_PORT"
echo ""
echo "Press Ctrl+C to stop"

# Wait for either process to exit
wait

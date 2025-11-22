# File: scripts/run_dev.sh
# ============================================
#!/bin/bash

# Start backend and frontend in parallel
trap 'kill 0' SIGINT  # Kill all background processes on Ctrl+C

echo " Starting development servers..."

# Backend
cd backend
export DATABASE_URL="postgres://eda_user:eda_pass@localhost/eda_platform"
export RUST_LOG=info
cargo run &
BACKEND_PID=$!

# Wait for backend to start
sleep 3

# Frontend
cd ../frontend
source venv/bin/activate 2>/dev/null || . venv/Scripts/activate
python main.py &
FRONTEND_PID=$!

echo "   Servers running:"
echo "   Backend PID: $BACKEND_PID"
echo "   Frontend PID: $FRONTEND_PID"
echo ""
echo "Press Ctrl+C to stop all servers"

wait
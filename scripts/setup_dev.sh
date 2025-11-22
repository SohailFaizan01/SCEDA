# File: scripts/setup_dev.sh
# ============================================
#!/bin/bash
set -e

echo " Setting up EDA Platform development environment..."

# Check prerequisites
command -v rustc >/dev/null 2>&1 || { echo " Rust not installed. Run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"; exit 1; }
command -v python3 >/dev/null 2>&1 || { echo " Python 3 not installed"; exit 1; }
command -v docker >/dev/null 2>&1 || { echo "  Docker not found. Install for easier database setup."; }

# Setup backend
echo " Setting up Rust backend..."
cd backend
cargo check
cd ..

# Setup frontend
echo " Setting up Python frontend..."
cd frontend
python3 -m venv venv
source venv/bin/activate || . venv/Scripts/activate  # Windows compatibility
pip install -r requirements.txt
cd ..

# Start database
if command -v docker >/dev/null 2>&1; then
    echo " Starting PostgreSQL..."
    docker-compose up -d postgres
    sleep 5  # Wait for postgres to start
    
    echo " Running migrations..."
    cd backend
    export DATABASE_URL="postgres://eda_user:eda_pass@localhost/eda_platform"
    cargo install sqlx-cli --no-default-features --features postgres
    sqlx migrate run
    cd ..
else
    echo "  Please setup PostgreSQL manually and run migrations"
fi

echo " Setup complete!"
echo ""
echo "To start development:"
echo "  1. Terminal 1: cd backend && cargo run"
echo "  2. Terminal 2: cd frontend && source venv/bin/activate && python main.py"
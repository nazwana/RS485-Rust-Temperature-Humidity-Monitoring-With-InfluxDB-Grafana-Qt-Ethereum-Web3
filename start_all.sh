#!/bin/bash
# Skrip untuk memulai semua layanan Sensor Gateway secara berurutan.

set -e

# --- PUSAT KONFIGURASI ---
BASE_DIR="/home/naufal/Documents/DATA/RUST"
HARDHAT_DIR="$BASE_DIR/hardhat-sensor-contract"
RUST_DIR="$BASE_DIR/sensor-gateway"
# Perbaikan: Nama folder React diganti ke 'frontend-dapp'
REACT_DIR="$BASE_DIR/frontend-dapp" 
PID_DIR="$BASE_DIR/pids"

# --- PERSIAPAN ---
mkdir -p "$PID_DIR"
echo "--- [$(date)] Memulai Semua Layanan Sensor Gateway ---"

# --- LANGKAH 1: MEMULAI HARDHAT NODE ---
echo "[1/5] Memulai Hardhat Node di background..."
cd "$HARDHAT_DIR"
npx hardhat node > "$BASE_DIR/hardhat-node.log" 2>&1 &
HARDHAT_PID=$!
echo $HARDHAT_PID > "$PID_DIR/hardhat.pid"
echo "      Hardhat Node berjalan dengan PID: $HARDHAT_PID"
sleep 5

# --- LANGKAH 2: DEPLOY SMART CONTRACT ---
echo "[2/5] Menjalankan deployment Smart Contract..."
npx hardhat run scripts/deploy.js --network localhost
echo "      Deployment selesai."

# --- LANGKAH 3: MEMULAI RUST BACKEND ---
echo "[3/5] Memulai Rust Backend..."
cd "$RUST_DIR"
if [ ! -f "./target/release/sensor-gateway" ]; then
    echo "      Membangun Rust project (mode release)... Ini mungkin butuh waktu."
    cargo build --release
fi
./target/release/sensor-gateway > "$BASE_DIR/rust-backend.log" 2>&1 &
RUST_PID=$!
echo $RUST_PID > "$PID_DIR/rust.pid"
echo "      Rust Backend berjalan dengan PID: $RUST_PID"
sleep 2

# --- LANGKAH 4: MEMULAI REACT FRONTEND ---
echo "[4/5] Memulai React Frontend..."
cd "$REACT_DIR"
# Perbaikan: Perintah 'npm start' digunakan di sini
npm start > "$BASE_DIR/react-frontend.log" 2>&1 & 
REACT_PID=$!
echo $REACT_PID > "$PID_DIR/react.pid"
echo "      React Frontend berjalan dengan PID: $REACT_PID"

echo "[5/5] ✅ Semua layanan telah dimulai."
echo "Log untuk setiap layanan disimpan di file .log di direktori $BASE_DIR"
echo "Untuk menghentikan semua, jalankan skrip stop_all.sh"

wait $HARDHAT_PID $RUST_PID $REACT_PID

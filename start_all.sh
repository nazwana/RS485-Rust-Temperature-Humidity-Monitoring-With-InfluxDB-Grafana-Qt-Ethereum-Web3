#!/bin/bash
# Skrip untuk memulai semua layanan Sensor Gateway secara berurutan.

# Keluar segera jika ada perintah yang gagal
set -e

# --- PUSAT KONFIGURASI ---
# Definisikan semua path direktori Anda di sini
BASE_DIR="/home/naufal/Documents/DATA/RUST"
HARDHAT_DIR="$BASE_DIR/hardhat-sensor-contract"
RUST_DIR="$BASE_DIR/sensor-gateaway"       
REACT_DIR="$BASE_DIR/frontend-dapp"  
PID_DIR="$BASE_DIR/pids"                  # Folder untuk menyimpan ID proses

# --- PERSIAPAN ---
# Pastikan folder PID ada untuk menyimpan ID proses yang berjalan
mkdir -p "$PID_DIR"
echo "--- [$(date)] Memulai Semua Layanan Sensor Gateway ---"

# --- LANGKAH 1: MEMULAI HARDHAT NODE ---
echo "[1/5] Memulai Hardhat Node di background..."
cd "$HARDHAT_DIR"
# Jalankan node dan simpan log-nya ke file, lalu jalankan di background (&)
npx hardhat node > "$BASE_DIR/hardhat-node.log" 2>&1 &
# Simpan Process ID (PID) dari proses terakhir yang dijalankan di background
HARDHAT_PID=$!
echo $HARDHAT_PID > "$PID_DIR/hardhat.pid"
echo "      Hardhat Node berjalan dengan PID: $HARDHAT_PID"
# Beri waktu beberapa detik agar node benar-benar siap
sleep 5

# --- LANGKAH 2: DEPLOY SMART CONTRACT ---
echo "[2/5] Menjalankan deployment Smart Contract..."
# Perintah ini akan berjalan dan ditunggu sampai selesai
npx hardhat run scripts/deploy.js --network localhost
echo "      Deployment selesai."

# --- LANGKAH 3: MEMULAI RUST BACKEND ---
echo "[3/5] Memulai Rust Backend..."
cd "$RUST_DIR"
# Compile aplikasi Rust Anda dalam mode rilis (lebih cepat) jika belum ada
# Ini hanya perlu dilakukan sekali kecuali ada perubahan kode
if [ ! -f "./target/release/sensor-gateway" ]; then
    echo "      Membangun Rust project (mode release)... Ini mungkin butuh waktu."
    cargo build --release
fi
# Jalankan binary yang sudah dioptimalkan di background
./target/release/sensor-gateway > "$BASE_DIR/rust-backend.log" 2>&1 &
RUST_PID=$!
echo $RUST_PID > "$PID_DIR/rust.pid"
echo "      Rust Backend berjalan dengan PID: $RUST_PID"
sleep 2

# --- LANGKAH 4: MEMULAI REACT FRONTEND ---
echo "[4/5] Memulai React Frontend..."
cd "$REACT_DIR"
# Jalankan server pengembangan React di background
npm start > "$BASE_DIR/react-frontend.log" 2>&1 &
REACT_PID=$!
echo $REACT_PID > "$PID_DIR/react.pid"
echo "      React Frontend berjalan dengan PID: $REACT_PID"

echo "[5/5] ✅ Semua layanan telah dimulai."
echo "Log untuk setiap layanan disimpan di file .log di direktori $BASE_DIR"
echo "Untuk menghentikan semua, jalankan skrip stop_all.sh"

# Tunggu semua proses background selesai (agar skrip tidak langsung keluar)
wait $HARDHAT_PID $RUST_PID $REACT_PID

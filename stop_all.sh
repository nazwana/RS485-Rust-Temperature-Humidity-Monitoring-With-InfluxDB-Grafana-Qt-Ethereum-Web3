#!/bin/bash
#
# Skrip untuk menghentikan semua layanan Sensor Gateway yang berjalan di background.
#

echo "--- Menghentikan Semua Layanan Sensor Gateway ---"

# Tentukan path direktori utama dan folder PID
BASE_DIR="/home/naufal/Documents/DATA/RUST"
PID_DIR="$BASE_DIR/pids"

# Fungsi pembantu untuk menghentikan proses dengan aman
kill_process() {
    local pid_file=$1
    local process_name=$2
    
    # Cek apakah file PID ada
    if [ -f "$pid_file" ]; then
        # Baca ID Proses dari file
        PID=$(cat "$pid_file")
        echo "Menghentikan $process_name (PID: $PID)..."
        
        # Coba hentikan prosesnya. `|| true` mencegah skrip berhenti jika proses sudah mati.
        kill $PID > /dev/null 2>&1 || true
        
        # Hapus file PID setelah selesai
        rm "$pid_file"
        echo "      $process_name telah dihentikan."
    else
        echo "$process_name sepertinya sudah tidak berjalan (file .pid tidak ditemukan)."
    fi
}

# Hentikan semua layanan dalam urutan terbalik dari startup
# Ini untuk memastikan layanan yang bergantung dihentikan terlebih dahulu
kill_process "$PID_DIR/react.pid" "React Frontend (npm start)"
kill_process "$PID_DIR/rust.pid" "Rust Backend (sensor-gateway)"
kill_process "$PID_DIR/hardhat.pid" "Hardhat Node"

echo "✅ Semua layanan telah dihentikan."

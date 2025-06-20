# RS485 Rust-Based Temperature & Humidity Monitoring with InfluxDB, Grafana, Qt, and Ethereum Blockchain Integration

Sistem ini dirancang untuk memantau suhu dan kelembapan secara real-time menggunakan sensor RS485 (SHT20/SHT40), dengan integrasi ke database time-series (InfluxDB), visualisasi (Grafana dan Qt), dan pencatatan ke blockchain Ethereum melalui smart contract berbasis Web3.

---

## 🎓 Proyek Kelompok 5

**Mata Kuliah:** Interkoneksi Sistem Instrumentasi  
**Departemen:** Teknik Instrumentasi  
**Fakultas:** Vokasi  
**Universitas:** Institut Teknologi Sepuluh Nopember (ITS)

### 👥 Anggota:
- Andik Putra Nazwana – 2042231010  
- Akhmad Maulvin Nazir Zakaria – 2042231028  
- Naufal Faqiih Ashshiddiq – 2042231068

**Dosen Pembimbing:** Ahmad Radhy, S.Si., M.Si

---

## ⚙️ Teknologi yang Digunakan

| Komponen        | Teknologi                           | Peran                                                       |
|----------------|--------------------------------------|-------------------------------------------------------------|
| **Sensor**      | SHT20 RS485 (Modbus RTU)             | Membaca suhu & kelembapan secara digital                   |
| **Gateway**     | Rust, tokio, ethers-rs               | Mengubah data Modbus ke TCP + mengirim ke smart contract    |
| **Visualisasi** | Grafana + InfluxDB + Qt (PySide6)    | Monitoring grafik real-time (web & desktop)                |
| **Blockchain**  | Ethereum, Solidity, Hardhat          | Mencatat hash data ke chain (immutable & traceable)         |
| **Frontend**    | React + Web3 + ethers.js + MetaMask  | Menampilkan data blockchain secara terbuka (DApp)           |

---

## 🔁 Penjelasan Alur Proyek

1. **Pembacaan Data Sensor**
   - SHT20/SHT40 mengirimkan data suhu dan kelembapan melalui protokol RS485 Modbus RTU.

2. **Gateway Rust**
   - Program Rust membaca data serial, mengirimnya ke:
     - Dashboard Qt (melalui TCP Socket)
     - Blockchain Ethereum (melalui `ethers-rs` ke smart contract)
     - InfluxDB (untuk histori dan Grafana)

3. **Visualisasi Desktop (Qt)**
   - GUI menggunakan PySide6:
     - Menampilkan tabel data real-time.
     - Menampilkan grafik line chart & bar chart.

4. **Time-Series Database**
   - InfluxDB menyimpan data suhu & kelembapan berdasarkan waktu.
   - Grafana membaca InfluxDB → visualisasi grafik web.

5. **Integrasi Blockchain**
   - Data dikirim ke Ethereum (Sepolia atau local Hardhat) via smart contract.
   - Fungsi `recordData(deviceId, temperature, humidity)` mencatat data ke blockchain.
   - Data ini immutable (tidak bisa diubah), transparan, dan dapat diverifikasi.

6. **Web3 DApp Frontend**
   - DApp dibuat dengan React & Web3.js:
     - Menghubungkan ke MetaMask (wallet pengguna).
     - Mendengarkan event `DataRecorded` dari smart contract.
     - Menampilkan data sensor dalam tabel dan grafik real-time.

---

## 🔐 Keunggulan Sistem

- ✅ **Real-time**: Data sensor dikirim dan divisualisasikan langsung.
- ✅ **Terdistribusi & Transparan**: Data dicatat di blockchain Ethereum.
- ✅ **Aman**: Hash data tidak bisa dimodifikasi (immutable).
- ✅ **Multiplatform**: Tersedia dashboard berbasis desktop (Qt) dan web (DApp).
- ✅ **Traceability**: Setiap data tercatat sebagai transaksi Ethereum.

---

## 📂 Struktur Folder

```bash
.
├── sensor-gateway/         # Program Rust (Modbus RTU → TCP + Smart Contract)
├── sensor-dashboard-qt/    # Dashboard PySide6 Qt GUI
├── influxdb/               # Konfigurasi database time-series
├── grafana/                # Dashboard visualisasi InfluxDB
├── hardhat-sensor-contract # Smart Contract Solidity + deployment Hardhat
├── frontend-dapp/          # Web3 DApp React + ethers.js + MetaMask
└── README.md               # Dokumentasi proyek ini

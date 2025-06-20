use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::time::sleep;
use anyhow::Result;
use serde::Serialize;
use futures::stream;

// Impor untuk Modbus (sesuai versi 0.9 yang terbukti bekerja)
use tokio_serial::{self, SerialPortBuilderExt};
use tokio_modbus::{
    prelude::*,
    Slave,
};

// Impor lainnya
use influxdb2::Client;
use influxdb2::models::DataPoint;
use chrono::Utc;
use ethers::prelude::*;
use ethers::abi::Abi;
use std::fs::File;
use std::io::BufReader as StdBufReader;
use std::env;

//================================================================//
// --- PUSAT KONFIGURASI ---
//================================================================//
const SERIAL_PORT_PATH: &str = "/dev/ttyUSB0";
const BAUD_RATE: u32 = 9600;
const SLAVE_ID: u8 = 1;
const START_REGISTER: u16 = 1;
const NUM_REGISTERS: u16 = 2;
const DEVICE_ID: &str = "device-01";
const TCP_SERVER_ADDRESS: &str = "127.0.0.1:8080";
const INFLUXDB_URL: &str = "http://localhost:8086";
const INFLUXDB_ORG: &str = "influxDB";
const INFLUXDB_BUCKET: &str = "sensor_data";
//================================================================//

/// Struct untuk data state bersama.
#[derive(Debug, Clone, Default, Serialize)]
struct AppData {
    // Data sensor real-time
    temperature: f32,
    humidity: f32,
    timestamp: i64,

    // Data blockchain yang akan diupdate oleh task writer
    contract_address: String,
    gateway_address: String,
    last_tx_hash: String,
    last_tx_status: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    println!("Aplikasi Sensor Gateway (Mode Otomatis) Dimulai...");
    
    let shared_data = Arc::new(Mutex::new(AppData::default()));

    // Inisialisasi data statis blockchain sekali saja
    {
        let contract_address = env::var("CONTRACT_ADDRESS").unwrap_or_default();
        let gateway_private_key = env::var("GATEWAY_PRIVATE_KEY").unwrap_or_default();
        let wallet: LocalWallet = gateway_private_key.parse()?;
        
        let mut data_guard = shared_data.lock().await;
        data_guard.contract_address = contract_address;
        data_guard.gateway_address = format!("{:?}", wallet.address());
    }

    // Jalankan semua tiga task utama secara bersamaan
    tokio::spawn(poll_sensor_data(Arc::clone(&shared_data)));
    tokio::spawn(run_tcp_server(Arc::clone(&shared_data)));
    tokio::spawn(run_database_writer(Arc::clone(&shared_data)));

    println!("\nSemua service telah dimulai dan berjalan di background:");
    println!("- ✅ Sensor Poller (Mode SENSOR ASLI)");
    println!("- ✅ TCP Server di {} (Menyiarkan data ke Qt/Web3)", TCP_SERVER_ADDRESS);
    println!("- ✅ Database & Blockchain Writer (Otomatis)");
    println!("\nAplikasi berjalan. Tekan Ctrl+C untuk berhenti.");

    // Tahan program utama agar tidak langsung keluar
    let _ = tokio::signal::ctrl_c().await;
    println!("\nMenutup semua service...");
    Ok(())
}

// --- TASK A: Sang Produsen (Membaca Sensor) ---
async fn poll_sensor_data(data: Arc<Mutex<AppData>>) -> Result<()> {
    let slave = Slave(SLAVE_ID);
    
    loop { // Loop tak terbatas untuk selalu mencoba koneksi ulang
        println!("[Sensor Poller] Mencoba koneksi ke sensor di {}...", SERIAL_PORT_PATH);
        let builder = tokio_serial::new(SERIAL_PORT_PATH, BAUD_RATE).timeout(Duration::from_secs(2));
        
        match builder.open_native_async() {
            Ok(port) => {
                let mut ctx = rtu::attach_slave(port, slave);
                println!("[Sensor Poller] Koneksi berhasil. Memulai pembacaan data...");
                
                // Loop pembacaan selama koneksi masih bagus
                loop {
                    match ctx.read_input_registers(START_REGISTER, NUM_REGISTERS).await {
                        Ok(response) if response.len() >= 2 => {
                            let temp = response[0] as f32 / 10.0;
                            let rh = response[1] as f32 / 10.0;
                            {
                                let mut data_guard = data.lock().await;
                                data_guard.temperature = temp;
                                data_guard.humidity = rh;
                                data_guard.timestamp = Utc::now().timestamp_nanos_opt().unwrap_or(0);
                            }
                            println!("[Sensor Poller] Data diperbarui: {:.1}°C, {:.1}%RH", temp, rh);
                        }
                        Ok(_) => eprintln!("[Sensor Poller] Menerima respons dengan panjang tidak valid."),
                        Err(e) => {
                            eprintln!("[Sensor Poller] Gagal membaca data (koneksi mungkin terputus): {}. Mencoba koneksi ulang...", e);
                            break; // Keluar dari loop pembacaan untuk mencoba koneksi ulang dari awal
                        }
                    }
                    sleep(Duration::from_secs(5)).await; // Jeda antar pembacaan
                }
            }
            Err(e) => {
                eprintln!("[Sensor Poller] Gagal membuka port serial: {}. Mencoba lagi dalam 10 detik...", e);
                sleep(Duration::from_secs(10)).await;
            }
        }
    }
}

// --- TASK B: Sang Pelayan (Menyiarkan Data via TCP) ---
async fn run_tcp_server(data: Arc<Mutex<AppData>>) -> Result<()> {
    let listener = TcpListener::bind(TCP_SERVER_ADDRESS).await?;
    loop {
        if let Ok((socket, _)) = listener.accept().await {
            tokio::spawn(handle_client(socket, Arc::clone(&data)));
        }
    }
}

async fn handle_client(socket: TcpStream, data: Arc<Mutex<AppData>>) {
    println!("[TCP Server] Klien terhubung.");
    let mut writer = BufReader::new(socket).into_inner();
    loop {
        let data_guard = data.lock().await;
        if data_guard.timestamp > 0 {
            if let Ok(json_string) = serde_json::to_string(&*data_guard) {
                if writer.write_all((json_string + "\n").as_bytes()).await.is_err() { break; }
                if writer.flush().await.is_err() { break; }
            }
        }
        drop(data_guard);
        sleep(Duration::from_secs(2)).await; // Kirim update setiap 2 detik
    }
    println!("[TCP Server] Klien terputus.");
}

// --- TASK C: Sang Pencatat (Menulis Otomatis ke DB & Blockchain) ---
async fn run_database_writer(data: Arc<Mutex<AppData>>) -> Result<()> {
    // Baca semua konfigurasi dari file .env
    let influx_token = env::var("INFLUXDB_TOKEN")?;
    let eth_rpc_url = env::var("ETH_RPC_URL")?;
    let contract_address = env::var("CONTRACT_ADDRESS")?;
    let gateway_private_key = env::var("GATEWAY_PRIVATE_KEY")?;

    // Inisialisasi semua klien
    let influx_client = Client::new(INFLUXDB_URL, INFLUXDB_ORG, &influx_token);
    let provider = Provider::<Http>::try_from(eth_rpc_url)?;
    let wallet = gateway_private_key.parse::<LocalWallet>()?.with_chain_id(provider.get_chainid().await?.as_u64());
    let signer = SignerMiddleware::new(provider, wallet.clone());
    let abi: Abi = serde_json::from_reader(StdBufReader::new(File::open("src/abi/DataRegistry.json")?))?;
    let contract = Contract::new(contract_address.parse::<Address>()?, abi, Arc::new(signer));

    let mut last_written_time = 0i64;

    loop {
        sleep(Duration::from_secs(10)).await; // Cek data baru setiap 10 detik
        let current_data: AppData;
        {
            let data_guard = data.lock().await;
            if data_guard.timestamp > last_written_time {
                current_data = data_guard.clone();
                last_written_time = current_data.timestamp;
            } else { continue; }
        }

        // Tulis ke InfluxDB
        let point = DataPoint::builder("realtime_sensor_data")
            .tag("device_id", DEVICE_ID)
            .field("temperature_celsius", current_data.temperature as f64)
            .field("humidity_percent", current_data.humidity as f64)
            .timestamp(current_data.timestamp).build()?;
        if let Err(e) = influx_client.write(INFLUXDB_BUCKET, stream::iter(vec![point])).await {
            eprintln!("[Writer] Gagal menulis ke InfluxDB: {}", e);
        } else { println!("[Writer] Berhasil menulis ke InfluxDB."); }

        // Tulis ke Blockchain (dijalankan di background agar tidak memblokir loop utama)
        tokio::spawn(send_blockchain_tx(Arc::clone(&data), contract.clone(), current_data));
    }
}

async fn send_blockchain_tx(shared_data: Arc<Mutex<AppData>>, contract: Contract<SignerMiddleware<Provider<Http>, Wallet<ethers::core::k256::ecdsa::SigningKey>>>, data_to_send: AppData) {
    let temp_for_chain = (data_to_send.temperature * 100.0) as i64;
    let hum_for_chain = (data_to_send.humidity * 100.0) as i64;
    
    // Update status di UI menjadi "Mengirim..."
    {
        let mut data_guard = shared_data.lock().await;
        data_guard.last_tx_status = "Mengirim...".to_string();
    }

    let call = match contract.method::<_, ()>("recordData", (DEVICE_ID.to_string(), temp_for_chain, hum_for_chain)) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[Blockchain TX] Gagal membuat panggilan kontrak: {}", e);
            return;
        }
    };
    
    let (tx_hash, status) = match call.send().await {
        Ok(pending_tx) => {
            let hash = format!("{:?}", pending_tx.tx_hash());
            println!("[Writer] Transaksi dikirim, menunggu konfirmasi... Hash: {}", hash);
            match pending_tx.await {
                Ok(Some(receipt)) => (hash, format!("Terkonfirmasi (Blok: {})", receipt.block_number.unwrap_or_default())),
                _ => (hash, "Gagal mendapatkan konfirmasi.".to_string()),
            }
        }
        Err(e) => ("-".to_string(), format!("Gagal mengirim: {}", e)),
    };
    
    println!("[Writer] Status Transaksi Final: {}", status);
    // Perbarui shared state dengan info transaksi final
    {
        let mut data_guard = shared_data.lock().await;
        data_guard.last_tx_hash = tx_hash;
        data_guard.last_tx_status = status;
    }
}
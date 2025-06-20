use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncWriteExt};
use tokio::time::sleep;
use anyhow::Result;
use serde::Serialize;
use rand::Rng;
use futures::stream;

// Impor komponen untuk Modbus (sesuai versi 0.9)
use tokio_serial::{self, SerialPortBuilderExt};
use tokio_modbus::{
    prelude::*,
    client::{Context, rtu},
    Slave,
};

// Impor komponen lainnya
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
// Konfigurasi Sensor Modbus
const SERIAL_PORT_PATH: &str = "/dev/ttyUSB0"; // Pastikan ini benar
const BAUD_RATE: u32 = 9600;
const SLAVE_ID: u8 = 1;
const START_REGISTER: u16 = 1; // Sesuai contoh Anda: mulai dari register 1
const NUM_REGISTERS: u16 = 2;   // Sesuai contoh Anda: baca 2 register

// Konfigurasi Service
const DEVICE_ID: &str = "device-01";
const TCP_SERVER_ADDRESS: &str = "127.0.0.1:8080";
const INFLUXDB_URL: &str = "http://localhost:8086";
const INFLUXDB_ORG: &str = "ITS";
const INFLUXDB_BUCKET: &str = "sensor_data";
const INFLUXDB_TOKEN: &str = "YiKoIkKU5AfcMgyu0urtRp7Ri42kGvYxyQccByE9B1Is05EEIE1Y5IGnMYMKX2YkE8dLYHKqAGUqhfAMWZ7twA==";
//================================================================//

#[derive(Debug, Clone, Default, Serialize)]
struct SensorData {
    temperature: f32,
    humidity: f32,
    timestamp: i64,
    // Data Blockchain yang akan ditambahkan oleh writer
    contract_address: String,
    gateway_address: String,
    last_tx_hash: String,
    last_tx_status: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    println!("File .env telah dimuat.");
    
    let shared_data = Arc::new(Mutex::new(SensorData::default()));
    let sensor_task_data = Arc::clone(&shared_data);
    let tcp_task_data = Arc::clone(&shared_data);
    let writer_task_data = Arc::clone(&shared_data);

    let sensor_handle = tokio::spawn(poll_sensor_data(sensor_task_data));
    let tcp_server_handle = tokio::spawn(run_tcp_server(tcp_task_data));
    let writer_handle = tokio::spawn(run_database_writer(writer_task_data));

    println!("\nSemua service telah dimulai dan berjalan di background:");
    println!("- ✅ Sensor Poller berjalan.");
    println!("- ✅ TCP Server di {}", TCP_SERVER_ADDRESS);
    println!("- ✅ Database & Blockchain Writer");
    println!("\nAplikasi berjalan. Tekan Ctrl+C untuk berhenti.");

    let _ = tokio::try_join!(sensor_handle, tcp_server_handle, writer_handle)?;
    Ok(())
}

/// TASK A: [VERSI DUMMY] Menghasilkan data sensor palsu secara periodik
async fn poll_sensor_data(data: Arc<Mutex<SensorData>>) -> Result<()> {
    
    // --- KODE MODBUS ASLI (DINONAKTIFKAN) ---
    // API BARU: Buka port serial secara async
    // let port = tokio_serial::new(SERIAL_PORT_PATH, BAUD_RATE).open_native_async()?;
    // // API BARU: Buat koneksi Modbus dari port yang sudah ada
    // let mut ctx = rtu::connect(port).await?;
    // ctx.set_slave(Slave(SLAVE_ID)); // API BARU: Set Slave ID setelah konek
    // --------------------------------------------

    println!("[Sensor Poller] Mode DUMMY aktif. Menghasilkan data palsu.");

    loop {
        // Buat scope terpisah hanya untuk menghasilkan angka acak
        let (temperature, humidity) = {
            let mut rng = rand::thread_rng();
            let temp = 25.0 + rng.gen_range(-1.5..1.5);
            let hum = 60.0 + rng.gen_range(-5.0..5.0);
            (temp, hum)
        }; // <-- `rng` dibuat dan langsung "dihancurkan" di sini, sebelum .await

        // --- Sekarang kita aman untuk melakukan .await ---

        // Kunci data untuk menulis (Logika ini tetap sama)
        let mut data_guard = data.lock().await; // .await pertama terjadi di sini
        data_guard.temperature = temperature;
        data_guard.humidity = humidity;
        data_guard.timestamp = Utc::now().timestamp_nanos_opt().unwrap_or(0);
        
        // Lepaskan lock secepat mungkin
        drop(data_guard);
        
        println!("[Sensor Poller] Data DUMMY diperbarui: {:.2}°C, {:.2}%RH", temperature, humidity);
        
        // Titik .await kedua
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}    

/// TASK A: Membaca data dari sensor Modbus asli (menggunakan API v0.9 yang terbukti)
// async fn poll_sensor_data(data: Arc<Mutex<SensorData>>) -> Result<()> {
//     let slave = Slave(SLAVE_ID);
//     println!("[Sensor Poller] aktif. Mencoba koneksi...");

//     // Buat koneksi sekali di luar loop
//     let builder = tokio_serial::new(SERIAL_PORT_PATH, BAUD_RATE)
//         .timeout(Duration::from_secs(1));
//     let port = builder.open_native_async()?;
//     let mut ctx = rtu::attach_slave(port, slave);
//     println!("[Sensor Poller] Koneksi ke sensor berhasil. Memulai pembacaan data...");
    
//     loop {
//         // Di dalam loop, kita hanya melakukan pembacaan
//         match ctx.read_input_registers(START_REGISTER, NUM_REGISTERS).await {
//             Ok(response) => {
//                 if response.len() == 2 {
//                     let temp = response[0] as f32 / 10.0;
//                     let rh = response[1] as f32 / 10.0;

//                     let mut data_guard = data.lock().await;
//                     data_guard.temperature = temp;
//                     data_guard.humidity = rh;
//                     data_guard.timestamp = Utc::now().timestamp_nanos_opt().unwrap_or(0);
//                     println!("[Sensor Poller] Data diperbarui: {:.1}°C, {:.1}%RH", temp, rh);
//                 } else {
//                     eprintln!("[Sensor Poller] Menerima respons dengan panjang tidak valid: {:?}", response);
//                 }
//             }
//             Err(e) => {
//                 eprintln!("[Sensor Poller] Gagal membaca data sensor: {}. Mencoba lagi...", e);
//                 // Jika error, coba koneksi ulang setelah jeda
//                 sleep(Duration::from_secs(5)).await;
//                 // Re-establish connection
//                 let builder = tokio_serial::new(SERIAL_PORT_PATH, BAUD_RATE).timeout(Duration::from_secs(1));
//                 if let Ok(port) = builder.open_native_async() {
//                     ctx = rtu::attach_slave(port, slave);
//                     println!("[Sensor Poller] Mencoba koneksi ulang...");
//                 }
//                 continue;
//             }
//         }
//         sleep(Duration::from_secs(2)).await;
//     }
// }

/// TASK B: Menjalankan TCP Server untuk menyajikan data dalam format JSON
async fn run_tcp_server(data: Arc<Mutex<SensorData>>) -> Result<()> {
    let listener = TcpListener::bind(TCP_SERVER_ADDRESS).await?;
    loop {
        let (mut stream, addr) = listener.accept().await?;
        println!("[TCP Server] Koneksi diterima dari: {}", addr);
        let data_clone = Arc::clone(&data);

        tokio::spawn(async move {
            let data_guard = data_clone.lock().await;
            let response = serde_json::to_string(&*data_guard).unwrap_or_default();
            
            if let Err(e) = stream.write_all(response.as_bytes()).await {
                eprintln!("[TCP Server] Gagal mengirim data: {}", e);
            }
        });
    }
}

/// TASK C: Sang Pencatat (Menulis ke DB dan Blockchain)
async fn run_database_writer(data: Arc<Mutex<SensorData>>) -> Result<()> {
    let eth_rpc_url = env::var("ETH_RPC_URL").expect("ETH_RPC_URL harus diset di .env");
    let contract_address = env::var("CONTRACT_ADDRESS").expect("CONTRACT_ADDRESS harus diset di .env");
    let gateway_private_key = env::var("GATEWAY_PRIVATE_KEY").expect("GATEWAY_PRIVATE_KEY harus diset di .env");

    let influx_client = Client::new(INFLUXDB_URL, INFLUXDB_ORG, INFLUXDB_TOKEN);
    let provider = Provider::<Http>::try_from(eth_rpc_url)?;
    let wallet = gateway_private_key.parse::<LocalWallet>()?.with_chain_id(provider.get_chainid().await?.as_u64());
    let signer = SignerMiddleware::new(provider, wallet.clone());
    let abi: Abi = serde_json::from_reader(StdBufReader::new(File::open("src/abi/DataRegistry.json")?))?;
    let contract = Contract::new(contract_address.parse::<Address>()?, abi, Arc::new(signer));

    let mut last_written_time = 0i64;

    loop {
        sleep(Duration::from_secs(10)).await;
        let current_data: SensorData;
        {
            let data_guard = data.lock().await;
            if data_guard.timestamp > last_written_time {
                current_data = data_guard.clone();
                last_written_time = current_data.timestamp;
            } else { continue; }
        }

        // Tulis ke InfluxDB
        let point = DataPoint::builder("environment_monitoring")
            .tag("device_id", DEVICE_ID)
            .field("temperature_celsius", current_data.temperature as f64)
            .field("humidity_percent", current_data.humidity as f64)
            .timestamp(current_data.timestamp)
            .build()?;
        if let Err(e) = influx_client.write(INFLUXDB_BUCKET, stream::iter(vec![point])).await {
            eprintln!("[Writer] Gagal menulis ke InfluxDB: {}", e);
        } else {
            println!("[Writer] Berhasil menulis data ke InfluxDB.");
        }

        // Tulis ke Blockchain
        let temp_for_chain = (current_data.temperature * 100.0) as i64;
        let hum_for_chain = (current_data.humidity * 100.0) as i64;

        let call = contract.method::<_, ()>("recordData", (DEVICE_ID.to_string(), temp_for_chain, hum_for_chain))?;
        let tx_result = call.send().await;
        match tx_result {
            Ok(pending_tx) => {
                if let Some(receipt) = pending_tx.await? {
                     println!("[Writer] Transaksi Blockchain terkonfirmasi! Hash: {:?}", receipt.transaction_hash);
                } else {
                     println!("[Writer] Transaksi Blockchain dikirim, menunggu konfirmasi...");
                }
            }
            Err(e) => {
                eprintln!("[Writer] Gagal mengirim transaksi Blockchain: {}", e);
            }
        }
    }
}
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::time::sleep;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use futures::stream;

// Impor untuk Modbus
use tokio_serial::{self, SerialPortBuilderExt};
use tokio_modbus::{
    prelude::*,
    client::Context,
    Slave,
};

// Impor untuk InfluxDB & Waktu
use influxdb2::Client;
use influxdb2::models::DataPoint;
use chrono::Utc;

// Impor untuk Blockchain (Ethers)
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
const INFLUXDB_ORG: &str = "ITS";
const INFLUXDB_BUCKET: &str = "sensor_data";
//================================================================//

/// Struct untuk data state bersama.
/// 'Deserialize' ditambahkan untuk membaca perintah dari Qt.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

/// Struct untuk perintah yang dikirim dari Qt
#[derive(Deserialize)]
struct QtCommand {
    command: String,
    data: AppData, // Qt akan mengirim kembali data yang ingin dijual
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    println!("Aplikasi Sensor Gateway (Mode Interaktif) Dimulai...");
    
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

    // Jalankan task poller sensor dan server TCP
    tokio::spawn(poll_sensor_data(Arc::clone(&shared_data)));
    tokio::spawn(run_tcp_server(Arc::clone(&shared_data)));

    println!("\nSemua service telah dimulai:");
    println!("- ✅ Sensor Poller (Mode SENSOR ASLI)");
    println!("- ✅ TCP Server di {} (Menunggu koneksi dari Qt)", TCP_SERVER_ADDRESS);
    println!("\nAplikasi berjalan. Tekan Ctrl+C untuk berhenti.");

    let _ = tokio::signal::ctrl_c().await;
    println!("\nMenutup service...");
    Ok(())
}


// --- TASK A: MEMBACA SENSOR ---
async fn poll_sensor_data(data: Arc<Mutex<AppData>>) -> Result<()> {
    let slave = Slave(SLAVE_ID);
    println!("[Sensor Poller] Siap membaca dari sensor fisik...");

    loop {
        println!("[Sensor Poller] Mencoba koneksi ke sensor di {}...", SERIAL_PORT_PATH);
        let builder = tokio_serial::new(SERIAL_PORT_PATH, BAUD_RATE).timeout(Duration::from_secs(2));
        
        if let Ok(port) = builder.open_native_async() {
            let mut ctx = rtu::attach_slave(port, slave);
            println!("[Sensor Poller] Koneksi berhasil. Memulai pembacaan data...");
            
            loop { // Loop pembacaan selama koneksi masih bagus
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
                        eprintln!("[Sensor Poller] Gagal membaca data: {}. Mencoba koneksi ulang...", e);
                        break;
                    }
                }
                sleep(Duration::from_secs(2)).await;
            }
        } else {
            eprintln!("[Sensor Poller] Gagal membuka port serial. Mencoba lagi dalam 10 detik...");
            sleep(Duration::from_secs(10)).await;
        }
    }
}


// --- TASK B: SERVER TCP DUA ARAH ---
async fn run_tcp_server(data: Arc<Mutex<AppData>>) -> Result<()> {
    let listener = TcpListener::bind(TCP_SERVER_ADDRESS).await?;
    loop {
        if let Ok((socket, _)) = listener.accept().await {
            println!("[TCP Server] Klien Qt terhubung.");
            tokio::spawn(handle_qt_client(socket, Arc::clone(&data)));
        }
    }
}

/// Fungsi untuk menangani satu koneksi klien Qt
async fn handle_qt_client(socket: TcpStream, data: Arc<Mutex<AppData>>) {
    println!("[TCP Server] Klien Qt terhubung.");
    let (reader, mut writer) = socket.into_split();
    let mut reader = BufReader::new(reader);
    let mut line_buffer = String::new();

    loop {
        tokio::select! {
            // Cabang ini akan mendengarkan perintah dari Qt
            result = reader.read_line(&mut line_buffer) => {
                if let Ok(bytes_read) = result {
                    if bytes_read == 0 { break; } // Klien terputus
                    if let Ok(command) = serde_json::from_str::<QtCommand>(&line_buffer) {
                        if command.command == "SELL" {
                            println!("[TCP Server] Menerima perintah JUAL.");
                            tokio::spawn(process_sell_request(command.data, Arc::clone(&data)));
                        }
                    }
                    line_buffer.clear();
                } else { break; }
            },
            // Cabang ini akan mengirim update data live ke Qt
            _ = sleep(Duration::from_secs(2)) => {
                let data_guard = data.lock().await;
                if data_guard.timestamp > 0 {
                    if let Ok(json_string) = serde_json::to_string(&*data_guard) {
                        if writer.write_all((json_string + "\n").as_bytes()).await.is_err() {
                            break;
                        }
                        if writer.flush().await.is_err() { break; }
                    }
                }
            }
        }
    }
    println!("[TCP Server] Klien Qt terputus.");
}

// --- FUNGSI EKSEKUTOR PERINTAH "JUAL" ---
async fn process_sell_request(data_to_sell: AppData, shared_data: Arc<Mutex<AppData>>) -> Result<()> {
    println!("[Processor] Memulai proses penulisan untuk data timestamp: {}", data_to_sell.timestamp);

    let influx_token = env::var("INFLUXDB_TOKEN")?;
    let eth_rpc_url = env::var("ETH_RPC_URL")?;
    let contract_address = env::var("CONTRACT_ADDRESS")?;
    let gateway_private_key = env::var("GATEWAY_PRIVATE_KEY")?;

    let influx_client = Client::new(INFLUXDB_URL, INFLUXDB_ORG, &influx_token);
    let provider = Provider::<Http>::try_from(eth_rpc_url)?;
    let wallet = gateway_private_key.parse::<LocalWallet>()?.with_chain_id(provider.get_chainid().await?.as_u64());
    let signer = SignerMiddleware::new(provider, wallet.clone());
    let abi: Abi = serde_json::from_reader(StdBufReader::new(File::open("src/abi/DataRegistry.json")?))?;
    let contract = Contract::new(contract_address.parse::<Address>()?, abi, Arc::new(signer));

    // Update status di shared state menjadi "Processing..."
    {
        let mut data_guard = shared_data.lock().await;
        data_guard.last_tx_hash = "Processing...".to_string();
        data_guard.last_tx_status = "Sending to destinations...".to_string();
    }

    // 1. Tulis ke InfluxDB
    let point = DataPoint::builder("sold_sensor_data")
        .tag("device_id", DEVICE_ID)
        .field("temperature_celsius", data_to_sell.temperature as f64)
        .field("humidity_percent", data_to_sell.humidity as f64)
        .timestamp(data_to_sell.timestamp).build()?;
    influx_client.write(INFLUXDB_BUCKET, stream::iter(vec![point])).await?;
    println!("[Processor] Berhasil menulis ke InfluxDB.");

    // 2. Tulis ke Blockchain
    let temp_for_chain = (data_to_sell.temperature * 100.0) as i64;
    let hum_for_chain = (data_to_sell.humidity * 100.0) as i64;
    let call = contract.method::<_, ()>("recordData", (DEVICE_ID.to_string(), temp_for_chain, hum_for_chain))?;
    
    let (tx_hash, status) = match call.send().await {
        Ok(pending_tx) => {
            let hash = format!("{:?}", pending_tx.tx_hash());
            println!("[Processor] Transaksi dikirim, menunggu konfirmasi... Hash: {}", hash);
            match pending_tx.await {
                Ok(Some(receipt)) => (hash, format!("Terkonfirmasi (Blok: {})", receipt.block_number.unwrap_or_default())),
                _ => (hash, "Gagal mendapatkan konfirmasi.".to_string()),
            }
        }
        Err(e) => ("-".to_string(), format!("Gagal mengirim: {}", e)),
    };
    
    println!("[Processor] Status Transaksi Final: {}", status);
    // Perbarui shared state dengan info transaksi final
    {
        let mut data_guard = shared_data.lock().await;
        data_guard.last_tx_hash = tx_hash;
        data_guard.last_tx_status = status;
    }
    
    Ok(())
}
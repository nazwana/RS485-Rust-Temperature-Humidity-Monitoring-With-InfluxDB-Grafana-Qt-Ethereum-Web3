import { useState, useEffect, useRef } from 'react';
import { ethers } from 'ethers';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts';
import * as XLSX from 'xlsx'; // Impor library untuk Excel

// Pastikan path ke file ABI ini benar
import DataRegistryABI from './abi/DataRegistry.json';

// --- KONFIGURASI ---
const contractAddress = process.env.REACT_APP_CONTRACT_ADDRESS;
const DEVICE_ID = "device-01";
const HARDHAT_RPC_URL = "http://127.0.0.1:8545";


// --- Komponen Baru untuk Modal ---
const Modal = ({ reading, onClose }) => {
  if (!reading) return null;

  const modalStyle = {
    position: 'fixed',
    top: '50%',
    left: '50%',
    transform: 'translate(-50%, -50%)',
    backgroundColor: 'white',
    padding: '30px',
    zIndex: 1000,
    borderRadius: '10px',
    boxShadow: '0 5px 15px rgba(0,0,0,0.3)',
    width: '90%',
    maxWidth: '600px',
  };

  const overlayStyle = {
    position: 'fixed',
    top: 0,
    left: 0,
    right: 0,
    bottom: 0,
    backgroundColor: 'rgba(0,0,0,0.7)',
    zIndex: 999,
  };

  const detailItemStyle = {
    margin: '10px 0',
    wordWrap: 'break-word',
  };

  return (
    <>
      <div style={overlayStyle} onClick={onClose} />
      <div style={modalStyle}>
        <h3 style={{ marginTop: 0, borderBottom: '1px solid #eee', paddingBottom: '10px' }}>Detail Transaksi Blockchain</h3>
        <p style={detailItemStyle}><strong>Waktu Sensor:</strong> {reading.time}</p>
        <p style={detailItemStyle}><strong>Hash Transaksi:</strong> <span style={{ fontFamily: 'monospace' }}>{reading.hash}</span></p>
        <p style={detailItemStyle}><strong>Nomor Blok:</strong> {reading.blockNumber}</p>
        <p style={detailItemStyle}><strong>Alamat Pengirim (Gateway):</strong> <span style={{ fontFamily: 'monospace' }}>{reading.gatewayAddress}</span></p>
        <button onClick={onClose} style={{ marginTop: '20px', padding: '10px 20px', cursor: 'pointer' }}>Tutup</button>
      </div>
    </>
  );
};


function App() {
  const [account, setAccount] = useState(null);
  const [readings, setReadings] = useState([]);
  
  // State baru untuk mengontrol modal
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [selectedReading, setSelectedReading] = useState(null);

  const listenerInitialized = useRef(false);

  // Fungsi untuk membuka modal
  const handleOpenModal = (readingData) => {
    setSelectedReading(readingData);
    setIsModalOpen(true);
  };

  // Fungsi untuk menutup modal
  const handleCloseModal = () => {
    setIsModalOpen(false);
    setSelectedReading(null);
  };

  // Fungsi untuk export ke Excel
  const handleExportToExcel = () => {
    // Siapkan data dengan header yang lebih deskriptif
    const dataToExport = readings.map(r => ({
      'Waktu Pencatatan': r.time,
      'Suhu (C)': r.temp,
      'Kelembapan (%RH)': r.hum,
      'Hash Transaksi': r.hash,
      'Nomor Blok': r.blockNumber,
      'Alamat Gateway': r.gatewayAddress
    }));

    const worksheet = XLSX.utils.json_to_sheet(dataToExport);
    const workbook = XLSX.utils.book_new();
    XLSX.utils.book_append_sheet(workbook, worksheet, "Data Sensor");

    // Buat nama file dengan timestamp saat ini
    const dateStr = new Date().toISOString().slice(0, 10);
    XLSX.writeFile(workbook, `DataSensor_${dateStr}.xlsx`);
  };

  const connectWallet = async () => {
    if (window.ethereum) {
      try {
        const accounts = await window.ethereum.request({ method: 'eth_requestAccounts' });
        setAccount(accounts[0]);
      } catch (error) {
        console.error("Gagal menghubungkan dompet:", error);
      }
    } else {
      alert("Harap instal MetaMask!");
    }
  };

  useEffect(() => {
    if (contractAddress && !listenerInitialized.current) {
      try {
        const provider = new ethers.JsonRpcProvider(HARDHAT_RPC_URL);
        const readOnlyContract = new ethers.Contract(contractAddress, DataRegistryABI, provider);

        const onDataRecorded = (deviceId, temperature, humidity, event) => {
          if (deviceId === DEVICE_ID) {
            // Buat objek data yang lebih lengkap
            const newReading = {
              temp: Number(temperature) / 100.0,
              hum: Number(humidity) / 100.0,
              time: new Date().toLocaleTimeString('id-ID', { hour: '2-digit', minute: '2-digit', second: '2-digit' }),
              hash: event.log.transactionHash,
              blockNumber: event.log.blockNumber,
              gatewayAddress: event.log.address, // Ini adalah alamat kontrak, bisa diganti jika perlu
            };
            
            setReadings(prevReadings => [...prevReadings.slice(-49), newReading]);
          }
        };

        readOnlyContract.on("DataRecorded", onDataRecorded);
        listenerInitialized.current = true;
        console.log("Listener 'DataRecorded' berhasil dipasang.");

        return () => {
          readOnlyContract.off("DataRecorded", onDataRecorded);
        };
      } catch (error) {
        console.error("Gagal memasang event listener:", error);
      }
    }
  }, []);

  return (
    <div className="App" style={{ fontFamily: 'sans-serif', maxWidth: '900px', margin: 'auto', padding: '20px' }}>
      {/* --- Komponen Modal --- */}
      <Modal reading={selectedReading} onClose={handleCloseModal} />

      <header className="App-header" style={{ marginBottom: '40px' }}>
        <h1>📊 Dashboard Sensor Blockchain</h1>
        {!account ? (
          <button onClick={connectWallet} style={{ padding: '10px 15px', fontSize: '16px', cursor: 'pointer' }}>
            Hubungkan Dompet MetaMask
          </button>
        ) : (
          <div style={{ background: '#f0f0f0', padding: '15px', borderRadius: '8px' }}>
            <p style={{ margin: '5px 0' }}><strong>Dompet Terhubung:</strong> {account}</p>
            <p style={{ margin: '5px 0' }}><strong>Alamat Kontrak:</strong> {contractAddress}</p>
          </div>
        )}
      </header>
      
      {account && (
        <main>
          <div style={{display: 'flex', justifyContent: 'space-between', alignItems: 'center'}}>
            <h2>Grafik Real-Time</h2>
            <button onClick={handleExportToExcel} disabled={readings.length === 0} style={{ padding: '8px 15px', cursor: 'pointer' }}>
              Export ke Excel
            </button>
          </div>
          <div style={{ width: '100%', height: 300, marginBottom: '40px' }}>
            <ResponsiveContainer>
              <LineChart data={readings} margin={{ top: 5, right: 30, left: 20, bottom: 5 }}>
                <CartesianGrid strokeDasharray="3 3" />
                <XAxis dataKey="time" />
                <YAxis />
                <Tooltip />
                <Legend />
                <Line type="monotone" dataKey="temp" name="Suhu (°C)" stroke="#ff7300" activeDot={{ r: 8 }} />
                <Line type="monotone" dataKey="hum" name="Kelembapan (%RH)" stroke="#387908" />
              </LineChart>
            </ResponsiveContainer>
          </div>
        
          <h2>Riwayat Data Sensor (Tabel)</h2>
          <div style={{ border: '1px solid #ccc', padding: '10px', minHeight: '200px', background: '#f9f9f9' }}>
            {readings.length === 0 ? (
              <p>Menunggu data masuk dari blockchain...</p>
            ) : (
              <table style={{ width: '100%', borderCollapse: 'collapse' }}>
                <thead>
                  <tr style={{ background: '#eee' }}>
                    <th style={{ padding: '8px', border: '1px solid #ddd', textAlign: 'left' }}>Waktu</th>
                    <th style={{ padding: '8px', border: '1px solid #ddd', textAlign: 'left' }}>Suhu (°C)</th>
                    <th style={{ padding: '8px', border: '1px solid #ddd', textAlign: 'left' }}>Kelembapan (%)</th>
                    <th style={{ padding: '8px', border: '1px solid #ddd', textAlign: 'left' }}>Aksi</th>
                  </tr>
                </thead>
                <tbody>
                  {[...readings].reverse().map((reading, index) => (
                    <tr key={index}>
                      <td style={{ padding: '8px', border: '1px solid #ddd' }}>{reading.time}</td>
                      <td style={{ padding: '8px', border: '1px solid #ddd' }}>{reading.temp.toFixed(2)}</td>
                      <td style={{ padding: '8px', border: '1px solid #ddd' }}>{reading.hum.toFixed(2)}</td>
                      <td style={{ padding: '8px', border: '1px solid #ddd' }}>
                        <button onClick={() => handleOpenModal(reading)}>Lihat Detail</button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </main>
      )}
    </div>
  );
}

export default App;
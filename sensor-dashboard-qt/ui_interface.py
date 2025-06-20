# main3.py (Versi Final yang Disesuaikan & Diperbaiki)

import sys
import json
from collections import deque
from PySide6.QtWidgets import (QApplication, QMainWindow, QTableWidgetItem, QTableWidget, 
                             QPushButton, QVBoxLayout, QTextEdit)
from PySide6.QtNetwork import QTcpSocket, QAbstractSocket
from PySide6.QtCore import QTimer, QByteArray, Slot, QDateTime, Qt, QPropertyAnimation, QEasingCurve
from PySide6.QtGui import QFont

# Impor library grafik yang sudah diinstal
import pyqtgraph as pg

# Impor kelas UI dari file yang baru kita kompilasi: ui_dashboard.py
from ui_dashboard import Ui_MainWindow

# --- Konfigurasi Awal Grafik ---
pg.setConfigOption('background', 'w')
pg.setConfigOption('foreground', 'k')


class MainWindow(QMainWindow):
    def __init__(self):
        super(MainWindow, self).__init__()

        self.ui = Ui_MainWindow()
        self.ui.setupUi(self)
        self.setWindowTitle("Dashboard Real-Time Sensor - Kelompok 1")

        # === PANGGIL SEMUA FUNGSI SETUP ===
        self.apply_styles()
        self.setup_sidebar_animation()
        self.setup_window_controls()
        self.setup_page_navigation()
        self.setup_page_content()
        self.setup_charts()
        
        # === LOGIKA TCP ANDA ===
        self.buffer = QByteArray()
        self.socket = QTcpSocket(self)
        self.socket.readyRead.connect(self.on_ready_read)
        self.socket.disconnected.connect(self.on_disconnected)
        self.socket.connected.connect(self.on_connected)

        self.connection_timer = QTimer(self)
        self.connection_timer.timeout.connect(self.try_to_connect)
        self.connection_timer.start(3000)
        self.try_to_connect()

    def apply_styles(self):
        sidebar_button_style = """
            QPushButton {
                background-color: transparent; color: #FFFFFF; border: 2px solid #4a5a6a;
                padding: 10px; text-align: left; border-radius: 8px; font: bold 12px;
            }
            QPushButton:hover { background-color: #4a5a6a; }
            QPushButton:pressed { background-color: #3a4a5a; }
        """
        self.ui.frame_4.setStyleSheet(sidebar_button_style)

        app_style = """
            QFrame#frame { background-color: #1e2b3a; }
            QWidget#centralwidget { background-color: #f0f0f0; }
            QLabel#label { color: white; font: bold 16px; }
            QLabel#label_3 { color: #cccccc; font: 12px; }
            QFrame { border: none; }
        """
        self.setStyleSheet(app_style)

    def setup_sidebar_animation(self):
        self.ui.pushButton_5.clicked.connect(self.toggle_menu)

    def setup_window_controls(self):
        self.ui.pushButton_6.clicked.connect(self.showMinimized)
        self.ui.pushButton_7.clicked.connect(self.close)
        self.ui.pushButton_8.clicked.connect(self.toggle_maximize_restore)

    def setup_page_navigation(self):
        self.ui.pushButton_3.clicked.connect(lambda: self.ui.stackedWidget.setCurrentWidget(self.ui.page))
        self.ui.pushButton_2.clicked.connect(lambda: self.ui.stackedWidget.setCurrentWidget(self.ui.page_2))
        self.ui.pushButton_4.clicked.connect(lambda: self.ui.stackedWidget.setCurrentWidget(self.ui.page_3))
        self.ui.pushButton.clicked.connect(lambda: self.ui.stackedWidget.setCurrentWidget(self.ui.page_4))
        self.ui.pushButton_5.clicked.connect(lambda: self.ui.stackedWidget.setCurrentWidget(self.ui.page))

    def setup_page_content(self):
        # --- HALAMAN 1: INFORMASI PROYEK ---
        info_layout = QVBoxLayout(self.ui.frame_15)
        info_text_edit = QTextEdit()
        info_text_edit.setReadOnly(True)
        info_text_edit.setStyleSheet("background-color: white; border: 1px solid #ccc; border-radius: 5px; padding: 10px;")
        info_text_edit.setHtml("""
            <h1><b>Informasi Proyek</b></h1>
            <p>Aplikasi ini dibuat oleh <b>Kelompok 1</b> sebagai bagian dari proyek untuk memonitor data sensor secara real-time.</p>
            <br><h3><b>Fitur Utama:</b></h3>
            <ul>
                <li>Menerima data suhu dan kelembapan dari server melalui TCP Socket.</li>
                <li>Menampilkan data dalam bentuk tabel yang diperbarui secara langsung.</li>
                <li>Visualisasi data dalam bentuk grafik garis (Line Chart) yang dinamis.</li>
                <li>Visualisasi data terkini dalam bentuk grafik batang (Bar Chart).</li>
            </ul>
        """)
        info_layout.removeWidget(self.ui.label_7)
        self.ui.label_7.deleteLater()
        info_layout.addWidget(info_text_edit)
        
        # --- HALAMAN 2: TABEL DATA (DIBUAT SECARA OTOMATIS) ---
        # Ini perbaikan penting: kita buat QTableWidget di sini karena tidak ada di file .ui
        self.ui.data_table = QTableWidget() 
        # Gunakan frame_17 di page_2 sebagai container tabel
        table_layout = QVBoxLayout(self.ui.frame_17)
        table_layout.addWidget(self.ui.data_table)
        
        # Setup kolom dan header tabel
        self.ui.data_table.setColumnCount(3)
        self.ui.data_table.setHorizontalHeaderLabels(["Waktu", "Suhu (°C)", "Kelembapan (%)"])
        self.ui.data_table.horizontalHeader().setStretchLastSection(True)

    def setup_charts(self):
        # --- GRAFIK GARIS (LINE CHART) ---
        self.plot_widget = pg.PlotWidget(title="Grafik Suhu & Kelembapan Real-Time")
        self.plot_widget.setLabel('left', 'Nilai Sensor')
        self.plot_widget.setLabel('bottom', 'Waktu (Sampel Data)')
        self.plot_widget.showGrid(x=True, y=True)
        self.plot_widget.addLegend()
        line_chart_layout = QVBoxLayout(self.ui.frame_19)
        line_chart_layout.addWidget(self.plot_widget)
        self.temp_line = self.plot_widget.plot(pen=pg.mkPen('r', width=2), name="Suhu (°C)")
        self.hum_line = self.plot_widget.plot(pen=pg.mkPen('b', width=2), name="Kelembapan (%)")
        self.max_points = 50
        self.time_data = deque(maxlen=self.max_points)
        self.temp_data = deque(maxlen=self.max_points)
        self.hum_data = deque(maxlen=self.max_points)
        self.x_axis_counter = 0

        # --- GRAFIK BATANG (BAR CHART) ---
        self.bar_widget = pg.PlotWidget(title="Data Sensor Terkini")
        self.bar_widget.setLabel('left', 'Nilai')
        bar_chart_layout = QVBoxLayout(self.ui.frame_21)
        bar_chart_layout.addWidget(self.bar_widget)
        self.bar_graph = pg.BarGraphItem(x=[1, 2], height=[0, 0], width=0.6, brushes=['r', 'b'])
        self.bar_widget.addItem(self.bar_graph)
        ticks = [(1, 'Suhu'), (2, 'Kelembapan')]
        ax = self.bar_widget.getAxis('bottom')
        ax.setTicks([ticks])

    def toggle_maximize_restore(self):
        if self.isMaximized(): self.showNormal()
        else: self.showMaximized()
            
    def toggle_menu(self):
        current_width = self.ui.frame.width()
        target_width = 80 if current_width == 250 else 250
        self.animation = QPropertyAnimation(self.ui.frame, b"minimumWidth")
        self.animation.setDuration(300)
        self.animation.setStartValue(current_width)
        self.animation.setEndValue(target_width)
        self.animation.setEasingCurve(QEasingCurve.Type.InOutCubic)
        self.animation.start()

    def update_charts(self, data):
        temp = data.get('temperature', 0)
        hum = data.get('humidity', 0)
        self.time_data.append(self.x_axis_counter)
        self.temp_data.append(temp)
        self.hum_data.append(hum)
        self.x_axis_counter += 1
        self.temp_line.setData(list(self.time_data), list(self.temp_data))
        self.hum_line.setData(list(self.time_data), list(self.hum_data))
        self.bar_graph.setOpts(height=[temp, hum])

    def try_to_connect(self):
        if self.socket.state() != QAbstractSocket.SocketState.ConnectedState:
            self.ui.label_6.setText("Mencoba terhubung...")
            self.socket.connectToHost("127.0.0.1", 8080)

    def on_connected(self):
        self.ui.label_6.setText("Terhubung & Menunggu Data")

    def on_disconnected(self):
        self.ui.label_6.setText("Koneksi terputus.")

    @Slot()
    def on_ready_read(self):
        self.buffer.append(self.socket.readAll())
        while self.buffer.contains(b'\n'):
            newline_pos = self.buffer.indexOf(b'\n')
            line_bytes = self.buffer.left(newline_pos + 1)
            self.buffer = self.buffer.mid(newline_pos + 1)
            try:
                json_str = str(line_bytes, 'utf-8').strip()
                if not json_str: continue
                live_data = json.loads(json_str)
                self.add_row_to_table(live_data)
                self.update_charts(live_data)
            except Exception as e:
                print(f"[PY ERROR] Gagal memproses data: {e}")

    def add_row_to_table(self, data):
        table = self.ui.data_table
        row_pos = table.rowCount()
        table.insertRow(row_pos)
        ts_nanos = data.get('timestamp', 0)
        ts_secs = ts_nanos // 1_000_000_000
        time_str = QDateTime.fromSecsSinceEpoch(int(ts_secs)).toString("yyyy-MM-dd HH:mm:ss")
        temp_str = f"{data.get('temperature', 0):.2f}"
        hum_str = f"{data.get('humidity', 0):.2f}"
        table.setItem(row_pos, 0, QTableWidgetItem(time_str))
        table.setItem(row_pos, 1, QTableWidgetItem(temp_str))
        table.setItem(row_pos, 2, QTableWidgetItem(hum_str))
        table.scrollToBottom()

if __name__ == "__main__":
    app = QApplication(sys.argv)
    window = MainWindow()
    window.show()
    sys.exit(app.exec())
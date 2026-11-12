use serialport::{SerialPort, FlowControl};
use std::io::Write;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Printer {
    port: Box<dyn SerialPort>,
}

impl Printer {
    /// Открывает COM3 на 115200, 8-N-1, Hardware flow control
    /// (из отчёта принтера: RS232 115200bps, 8 bits, parity none, Hardware handshaking).
    pub fn open(port_name: &str) -> Result<Self, serialport::Error> {
        let port = serialport::new(port_name, 115_200)
            .timeout(std::time::Duration::from_millis(500))
            .flow_control(FlowControl::Hardware)
            .open()?;
        Ok(Self { port })
    }

    /// Печать готового ESC/POS-буфера.
    /// ESC @ (reset) → пауза → bytes → пауза → flush.
    /// bytes уже содержит текст, LF×3 и GS V A 0 (cut) — см. escpos.rs.
    pub fn print(&mut self, bytes: &[u8]) -> Result<(), std::io::Error> {
        // 1. ESC @ — инициализация (сброс настроек, очистка буфера)
        self.port.write_all(&[0x1B, 0x40])?;
        self.port.flush()?;
        std::thread::sleep(std::time::Duration::from_millis(100));

        // 2. Тело чека (текст + LF + cut)
        self.port.write_all(bytes)?;
        self.port.flush()?;
        std::thread::sleep(std::time::Duration::from_millis(100));

        // 3. Финальный flush; пауза, чтобы принтер закончил печать
        self.port.flush()?;
        std::thread::sleep(std::time::Duration::from_millis(500));
        Ok(())
    }
}

pub type SharedPrinter = Arc<Mutex<Printer>>;
use serde::Deserialize;
use encoding_rs::IBM866;

#[derive(Debug, Deserialize)]
pub struct ReceiptRequest {
    pub order_id: String,
    pub machine_id: String,
    pub program: String,
    pub duration_min: i32,
    pub amount_rub: f64,
    pub payment_method: String,
    pub paid_at: String,
    pub company_name: String,
    pub company_inn: String,
    pub company_address: String,
    pub qr_data: String,
}

/// Кодирует строку в CP866 (IBM866).
fn cp866(s: &str) -> Vec<u8> {
    // Заменяем символы, которых нет в CP866, на безопасные аналоги
    let sanitized = s.replace('«', "\"").replace('»', "\"").replace('—', "-").replace('–', "-");

    let (bytes, _, _) = IBM866.encode(&sanitized);
    bytes.to_vec()
}

/// Разбивает строку на строки, не превышающие max_chars, стараясь не разрывать слова.
fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.len() + word.len() + 1 > max_chars && !current_line.is_empty() {
            lines.push(current_line.trim().to_string());
            current_line = String::new();
        }
        current_line.push_str(word);
        current_line.push(' ');
    }
    if !current_line.is_empty() {
        lines.push(current_line.trim().to_string());
    }
    lines
}

pub fn render_receipt(req: &ReceiptRequest) -> Vec<u8> {
    let mut out = Vec::new();

    // Инициализация
    out.extend_from_slice(&[0x1b, 0x40]);

    // Установка CP866
    out.extend_from_slice(&[0x1b, 0x74, 0x11]);

    // --- НАЗВАНИЕ КОМПАНИИ (ЦЕНТР, КРУПНО) ---
    out.extend_from_slice(&[0x1b, 0x61, 0x01]); // Center
    out.extend_from_slice(&[0x1b, 0x21, 0x38]); // Double height (крупный)

    let company_lines = wrap_text(&req.company_name, 22);
    for line in company_lines {
        out.extend_from_slice(&cp866(&line));
        out.extend_from_slice(b"\n");
    }

    out.extend_from_slice(&[0x1b, 0x21, 0x00]); // Сброс размера шрифта
    out.extend_from_slice(&[0x1b, 0x61, 0x00]); // Left align для остального текста

    // ── ИНН и адрес (слева, обычный шрифт) ──
    if !req.company_inn.is_empty() {
        out.extend_from_slice(&cp866(&format!("ИНН: {}", req.company_inn)));
        out.extend_from_slice(b"\n");
    }
    if !req.company_address.is_empty() {
        for line in wrap_text(&req.company_address, 32) {
            out.extend_from_slice(&cp866(&line));
            out.extend_from_slice(b"\n");
        }
    }

    out.extend_from_slice(b"------------------------------\n");

    // --- ОСНОВНОЙ ТЕКСТ ---
    out.extend_from_slice(&cp866("Заказ:    "));
    out.extend_from_slice(format!("{}\n", &req.order_id[..(8).min(req.order_id.len())]).as_bytes());

    out.extend_from_slice(&cp866("Машина:   "));
    out.extend_from_slice(format!("{}\n", req.machine_id).as_bytes());

    out.extend_from_slice(&cp866("Программа: "));
    out.extend_from_slice(&cp866(&req.program));
    out.extend_from_slice(b"\n");

    out.extend_from_slice(&cp866("Длительность: "));
    out.extend_from_slice(format!("{} ", req.duration_min).as_bytes());
    out.extend_from_slice(&cp866("мин\n"));

    out.extend_from_slice(&cp866("Сумма:    "));
    out.extend_from_slice(format!("{:.2} ", req.amount_rub).as_bytes());
    out.extend_from_slice(&cp866("руб.\n"));

    out.extend_from_slice(&cp866("Оплата:   "));
    if req.payment_method == "cash" {
        out.extend_from_slice(&cp866("Наличные\n"));
    } else {
        out.extend_from_slice(&cp866("Карта\n"));
    }

    out.extend_from_slice(&cp866("Время:    "));
    out.extend_from_slice(req.paid_at.as_bytes());
    out.extend_from_slice(b"\n");

    out.extend_from_slice(b"------------------------------\n");

    // --- QR-КОД ---
    // Выравнивание уже встроено в саму функцию render_qr_escpos.
    // Команду ESC a 1 здесь ставить НЕ НУЖНО, она игнорируется для растровых картинок.
    if let Ok(qr) = crate::qr::render_qr_escpos(&req.qr_data) {
        out.extend_from_slice(&qr);
    }

    out.extend_from_slice(b"\n\n\n");

    // Отрезка и выдвижение
    out.extend_from_slice(&[0x1b, 0x69]); // ESC i (Cut)
    out.extend_from_slice(&[0x1d, 0x65, 0x03, 0x0c]); // GS e (Eject)

    out
}

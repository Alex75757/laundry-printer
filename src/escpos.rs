use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ReceiptRequest {
    pub order_id: String,
    pub machine_id: String,
    pub program: String,
    pub duration_min: i32,
    pub amount_rub: f64,
    pub payment_method: String, // "card" | "cash"
    pub paid_at: String,
}

/// Рендер чека в ESC/POS (Vec<u8>).
/// Содержит: текст, LF×3, GS V A 0 (cut). ESC @ добавляет printer.rs::print.
pub fn render_receipt(req: &ReceiptRequest) -> Vec<u8> {
    let mut out = Vec::new();

    // Центрирование заголовка: ESC a 1
    out.extend_from_slice(b"\x1B\x61\x01");
    out.extend_from_slice(b"LAUNDRY\n");
    // Выравнивание влево: ESC a 0
    out.extend_from_slice(b"\x1B\x61\x00");
    out.extend_from_slice(b"------------------------------\n");

    let short_id = &req.order_id[..8.min(req.order_id.len())];
    out.extend_from_slice(format!("Order:    {}\n", short_id).as_bytes());
    out.extend_from_slice(format!("Machine:  {}\n", req.machine_id).as_bytes());
    out.extend_from_slice(format!("Program:  {}\n", req.program).as_bytes());
    out.extend_from_slice(format!("Duration: {} min\n", req.duration_min).as_bytes());
    out.extend_from_slice(format!("Amount:   {:.2} RUB\n", req.amount_rub).as_bytes());
    out.extend_from_slice(
        format!(
            "Payment:  {}\n",
            if req.payment_method == "cash" { "CASH" } else { "CARD" }
        )
        .as_bytes(),
    );
    out.extend_from_slice(format!("Paid at:  {}\n", req.paid_at).as_bytes());
    out.extend_from_slice(b"------------------------------\n");
    out.extend_from_slice(b"Thank you!\n\n\n");

    // LF × 3 (на всякий случай, если текст не заканчивается \n\n\n)
    out.extend_from_slice(&[0x0A, 0x0A, 0x0A]);
    // GS V A 0 — отрезка (стандарт ESC/POS)
    out.extend_from_slice(&[0x1D, 0x56, 0x41, 0x00]);

    out
}
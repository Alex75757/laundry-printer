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
    pub company_name: String, // «название компании»
    pub qr_data: String,
}

pub fn render_receipt(req: &ReceiptRequest) -> Vec<u8> {
    let mut out = Vec::new();

    // Компания (центр, крупно — ESC ! 0x38 = double width/height)
    out.extend_from_slice(b"\x1B\x61\x01\x1B\x21\x38");
    out.extend_from_slice(req.company_name.as_bytes());
    out.extend_from_slice(b"\x1B\x21\x00\n");   // обычный шрифт
    out.extend_from_slice(b"\x1B\x61\x00");       // left

    out.extend_from_slice(b"------------------------------\n");
    out.extend_from_slice(format!("Order:    {}\n", &req.order_id[..8.min(req.order_id.len())]).as_bytes());
    out.extend_from_slice(format!("Machine:  {}\n", req.machine_id).as_bytes());
    out.extend_from_slice(format!("Program:  {}\n", req.program).as_bytes());
    out.extend_from_slice(format!("Duration: {} min\n", req.duration_min).as_bytes());
    out.extend_from_slice(format!("Amount:   {:.2} RUB\n", req.amount_rub).as_bytes());
    out.extend_from_slice(format!("Payment:  {}\n", if req.payment_method == "cash" { "CASH" } else { "CARD" }).as_bytes());
    out.extend_from_slice(format!("Paid at:  {}\n", req.paid_at).as_bytes());   // время
    out.extend_from_slice(b"------------------------------\n");

    // QR-код (ESC/POS GS ( k)
    if let Ok(qr) = crate::qr::render_qr_escpos(&req.qr_data) {
        out.extend_from_slice(&qr);
    }

    out.extend_from_slice(b"\n\n\n");
    out.extend_from_slice(&[0x1D, 0x56, 0x41, 0x00]); // cut
    out
}
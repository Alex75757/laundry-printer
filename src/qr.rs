use qrcode::QrCode;
use qrcode::types::Color;

/// Генерирует ESC/POS байты для печати QR-кода как растрового изображения (GS v 0).
/// QR-код автоматически центрируется на бумаге шириной 576 точек (80 мм).
pub fn render_qr_escpos(data: &str) -> Result<Vec<u8>, String> {
    // 1. Генерируем матрицу QR-кода
    let code = QrCode::new(data.as_bytes()).map_err(|e| format!("QR encode failed: {e}"))?;
    let width = code.width();
    let height = code.width();

    // 2. Масштаб (размер модуля). 7 или 8 — хороший размер для 80мм.
    let scale = 7u32;
    let qr_pixel_w = (width as u32) * scale;
    let qr_pixel_h = (height as u32) * scale;

    // 3. Вычисляем отступы для центрирования
    // Стандартная ширина печати для 80мм ~ 576 точек.
    let paper_width_dots = 576u32;
    
    // Считаем отступ (делим на 2) и округляем вниз до кратного 8
    // (это гарантирует, что начало QR попадёт ровно на границу байта, 
    // что часто решает проблему с белой полосой при печати).
    let left_margin = if paper_width_dots > qr_pixel_w {
        ((paper_width_dots - qr_pixel_w) / 2) / 8 * 8
    } else {
        0
    };

    // Общая ширина картинки = отступ + QR + отступ справа (для симметрии)
    // Округляем итоговую ширину вверх до кратного 8.
    let total_img_w = left_margin + qr_pixel_w;
    let bytes_per_row = ((total_img_w + 7) / 8) as u16;
    let padded_img_w = (bytes_per_row as u32) * 8; // Реальная ширина в пикселях, кратная 8

    let img_h = qr_pixel_h;
    
    // 4. Формируем данные для принтера
    let mut data_bytes = Vec::with_capacity((bytes_per_row as usize) * (img_h as usize));

    for y in 0..img_h {
        let mut row = vec![0u8; bytes_per_row as usize];
        for x in 0..padded_img_w {
            // Проверяем, попадает ли текущий пиксель в область QR-кода
            if x >= left_margin && x < (left_margin + qr_pixel_w) {
                let qr_x = x - left_margin;
                let module_x = (qr_x / scale) as usize;
                let module_y = (y / scale) as usize;
                
                if code[(module_x, module_y)] == Color::Dark {
                    let byte_idx = (x / 8) as usize;
                    let bit_idx = 7 - (x % 8);
                    row[byte_idx] |= 1 << bit_idx;
                }
            }
            // Если x < left_margin (отступ слева) или x > QR (отступ справа), 
            // пиксель остается белым (0).
        }
        data_bytes.extend_from_slice(&row);
    }

    // 5. Собираем команду GS v 0
    let mut out = Vec::new();
    out.extend_from_slice(&[0x1D, 0x76, 0x30, 0x00]); // GS v 0 m=0 (normal)
    out.extend_from_slice(&[(bytes_per_row & 0xFF) as u8, (bytes_per_row >> 8) as u8]); // xL, xH
    out.extend_from_slice(&[(img_h & 0xFF) as u8, ((img_h >> 8) & 0xFF) as u8]);       // yL, yH
    out.extend_from_slice(&data_bytes);

    // Подача после QR
    out.extend_from_slice(&[0x0A, 0x0A]);
    Ok(out)
}
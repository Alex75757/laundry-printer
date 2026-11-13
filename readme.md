# laundry-printer

HTTP-сервис печати чеков для `laundry-stats`. Принимает `POST /print` с данными заказа, формирует ESC/POS-буфер (текст + QR + cut) и отправляет его в принтер через **COM-порт** (RS-232, `serialport`). Возвращает результат печати.

Сервис запускается **на хосте, к которому физически подключён принтер** (Windows или Linux). `laundry-stats` обращается к нему по HTTP. Принтер подключается по **последовательному порту** (`COM3` по умолчанию), настройки порта — `115200`, 8-N-1, Hardware flow control.

**Работает без настройки.** Порт, скорость и адрес сервиса захардкожены в коде (`main.rs`, `printer.rs`). Переменные окружения не используются.

## Требования

- Rust 1.75+ (для сборки) или готовый бинарник
- Принтер ESC/POS с **RS-232 (COM) интерфейсом**, поддерживающий кириллицу в CP866
- Свободный COM-порт (по умолчанию `COM3`), физически подключённый к принтеру
- Свободный порт `3001` для HTTP
- Настройки принтера (из отчёта устройства): `RS232 115200 bps, 8 bits, parity none, Hardware handshaking`

## Быстрый старт

```bash
cargo build --release
./target/release/laundry-printer
```

Сервис:

- открывает `COM3` на `115200`, 8-N-1, Hardware flow control;
- слушает `0.0.0.0:3001`;
- печатает на принтер, подключённый к `COM3`.

Проверка:

```bash
curl http://localhost:3001/health
# {"status":"ok","service":"laundry-printer"}
```

## Настройки (захардкожены)

| Параметр | Значение | Где в коде |
|---|---|---|
| COM-порт | `COM3` | `main.rs`: `let port_name = "COM3";` |
| Скорость | `115200` | `printer.rs`: `serialport::new(port_name, 115_200)` |
| Формат | 8-N-1 | `serialport` по умолчанию |
| Flow control | Hardware (RTS/CTS) | `printer.rs`: `.flow_control(FlowControl::Hardware)` |
| Таймаут записи | 500 мс | `printer.rs`: `.timeout(...)` |
| HTTP-адрес | `0.0.0.0:3001` | `main.rs`: `let addr = "0.0.0.0:3001";` |
| Кодировка | CP866 (IBM866) | `escpos.rs`: `encoding_rs::IBM866` |
| Ширина бумаги (для QR) | 576 точек (80 мм) | `qr.rs`: `paper_width_dots = 576` |
| Масштаб QR | 7 | `qr.rs`: `scale = 7` |
| Размер шрифта заголовка | Двойная высота | `escpos.rs`: `0x1B 0x21 0x38` |
| Отрезка | `ESC i` + `GS e 0x03 0x0C` | `escpos.rs` |

Чтобы изменить порт/скорость/адрес — правьте соответствующие константы в `main.rs` / `printer.rs` и пересобирайте.

## HTTP API

### `POST /print`

Печатает чек. Вызывается из `laundry-stats` (`receipt_client.rs`), не из браузера.

**Request:**

```json
{
  "order_id": "550e8400-e29b-41d4-a716-446655440000",
  "machine_id": "w6",
  "program": "Умеренное загрязнение",
  "duration_min": 45,
  "amount_rub": 400.0,
  "payment_method": "card",
  "paid_at": "2026-11-13T18:42:05+03:00",
  "company_name": "Прачечная",
  "qr_data": "ORDER:550e8400-e29b-41d4-a716-446655440000"
}
```

| Поле | Тип | Обязательно | Описание |
|---|---|---|---|
| `order_id` | string | да | ID заказа (на чеке печатаются первые 8 символов) |
| `machine_id` | string | да | ID машины (`w1`, `w6`, ...) |
| `program` | string | да | **Название** программы (label), а не id |
| `duration_min` | int | да | Длительность, минуты |
| `amount_rub` | float | да | Сумма, ₽ (печатается с 2 знаками) |
| `payment_method` | string | да | `card` / `cash` |
| `paid_at` | string (RFC3339) | да | Время оплаты в `RECEIPT_TZ` (`laundry-stats`), с offset |
| `company_name` | string | да | Название компании (печатается крупно, по центру, с переносом по 22 символа) |
| `qr_data` | string | да | Данные для QR-кода |

**Response `200`:**

```json
{ "ok": true, "order_id": "550e8400-e29b-41d4-a716-446655440000" }
```

**Response `500`:**

```json
"<io error message>"
```

| Код | Причина |
|---|---|
| `200` | Чек отправлен в порт |
| `500` | Ошибка записи в COM-порт (принтер offline, порт занят, нет связи) |

`laundry-stats` при `500` логирует `warn` и **не** ломает оплату: заказ остаётся `paid`, киоск показывает ошибку печати и кнопку повтора.

### `GET /health`

Проверка живости сервиса (порт не проверяется).

```json
{ "status": "ok", "service": "laundry-printer" }
```

## Формат чека

Рендерится в `escpos.rs` (`render_receipt`). Пример (80 мм):

```
================================================
                ПРАЧЕЧНАЯ
================================================
Заказ:    550e8400
Машина:   w6
Программа: Умеренное загрязнение
Длительность: 45 мин
Сумма:    400.00 руб.
Оплата:   Карта
Время:    2026-11-13T18:42:05+03:00
------------------------------
[QR: ORDER:550e8400-...]
```

Детали:

- **Заголовок** (`company_name`): по центру, двойная высота, перенос по 22 символам.
- **Основной текст**: слева, CP866, с выравниванием по меткам (`Заказ:`, `Машина:`, ...).
- **`order_id`**: печатаются **первые 8 символов** (или меньше, если короче).
- **`amount_rub`**: `{:.2}` + `руб.`
- **`payment_method`**: `cash` → «Наличные», иначе → «Карта».
- **`paid_at`**: печатается **как есть**, без конвертации (уже в `RECEIPT_TZ` от `laundry-stats`).
- **QR**: растровое изображение через `GS v 0`, центрируется на 576 точках, масштаб 7, отступ кратен 8 пикселям.
- **Отрезка**: `ESC i` (cut) + `GS e 0x03 0x0C` (eject).
- **Кодировка**: CP866 (`IBM866`); символы `«»—–` заменяются на `"` и `-`.

## Как это работает

1. `main.rs` при старте вызывает `Printer::open("COM3")` → `serialport::new(port_name, 115_200).flow_control(Hardware).open()`.
2. `POST /print` → `render_receipt(&req)` собирает ESC/POS-буфер:
   - `ESC @` — инициализация;
   - `ESC t 0x11` — CP866;
   - заголовок (center, double height);
   - строки заказа;
   - `GS v 0` — QR;
   - `ESC i` + `GS e` — cut/eject.
3. `Printer::print(&bytes)`:
   - `ESC @` + flush + 100 мс;
   - тело чека + flush + 100 мс;
   - финальный flush + 500 мс (чтобы принтер допечатал).
4. Принтер печатает и отрезает чек.

## Интеграция с `laundry-stats`

В `.env` `laundry-stats`:

```env
LAUNDRY_PRINTER_URL=http://laundry-printer:3001
COMPANY_NAME=Прачечная
RECEIPT_TZ=Europe/Moscow
```

> `COMPANY_NAME` в `.env` `laundry-stats` используется как fallback, но `receipt_client.rs` **всегда** передаёт `company_name` в `POST /print` (из `COMPANY_NAME` в `.env` `laundry-stats`). `laundry-printer` использует пришедшее значение.

Если `laundry-printer` запущен на другом хосте (не в Docker-сети), используйте IP: `http://192.168.2.143:3001`.

Проверка из контейнера `app`:

```powershell
docker compose exec app curl -s http://192.168.2.143:3001/health
# {"status":"ok","service":"laundry-printer"}
```

Проверка печати (вручную, для отладки):

```powershell
curl.exe -X POST http://192.168.2.143:3001/print `
  -H "Content-Type: application/json" `
  -d '{
    "order_id": "test-0001",
    "machine_id": "w6",
    "program": "Умеренное загрязнение",
    "duration_min": 45,
    "amount_rub": 400.0,
    "payment_method": "card",
    "paid_at": "2026-11-13T18:42:05+03:00",
    "company_name": "Прачечная",
    "qr_data": "ORDER:test-0001"
  }'
```

## Запуск как служба

### Windows (NSSM)

```powershell
nssm install laundry-printer "C:\laundry\laundry-printer.exe"
nssm start laundry-printer
```

Порт/скорость/адрес захардкожены в бинарнике. Чтобы изменить — пересобрать.

### Linux (systemd)

```ini
# /etc/systemd/system/laundry-printer.service
[Unit]
Description=Laundry receipt printer
After=network.target

[Service]
ExecStart=/opt/laundry-printer/laundry-printer
Restart=always
User=laundry
# доступ к /dev/ttyUSB0 (если COM-порт через USB-адаптер)
SupplementaryGroups=dialout

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl enable --now laundry-printer
sudo systemctl status laundry-printer
```

> На Linux `COM3` в коде нужно заменить на `/dev/ttyUSB0` или `/dev/ttyS0` и пересобрать.

## Troubleshooting

| Симптом | Причина | Решение |
|---|---|---|
| `Printer::open` → ошибка на старте | COM3 занят / не существует / нет прав | Проверить «Диспетчер устройств» → «Порты (COM и LPT)»; закрыть другие программы, использующие COM3; на Linux — `dialout` |
| `Permission denied` (Linux) | Нет доступа к `/dev/ttyUSB0` | `sudo usermod -aG dialout $USER`, перелогиниться |
| `500` на `POST /print` | Ошибка записи в порт (offline, нет бумаги, обрыв) | Проверить принтер, кабель, питание; логи `laundry-printer` |
| Кириллица печатается `????` | Принтер не в CP866 / прошивка не поддерживает | Проверить `ESC t 0x11`; возможно, нужен `cp1251` (правка `escpos.rs`) |
| Чек «обрезан» справа | Ширина бумаги не 80 мм / `paper_width_dots` неверный | Для 58 мм — `paper_width_dots = 384` в `qr.rs`, `PRINTER_WIDTH` в `escpos.rs` |
| QR не печатается | Принтер не поддерживает `GS v 0` | Проверить спецификацию принтера; QR можно отключить, убрав блок из `render_receipt` |
| Двойной сдвиг времени на чеке | Сервис конвертирует `paid_at` | **Не** конвертировать: печатать `paid_at` как есть (он уже в `RECEIPT_TZ` от `app`) |
| `program` = `medium` вместо названия | `laundry-stats` передал id | Проверить `program_label` в `app`; в `POST /print` должен быть label |
| `connection refused` из `app` | `LAUNDRY_PRINTER_URL` неверный / сервис не запущен | `curl.exe http://<host>:3001/health` с хоста `app` |
| Порт открылся, но чек не печатается | Неверный flow control / скорость | Принтер должен быть `115200, 8-N-1, Hardware`; иначе править `printer.rs` |

## Структура

```
laundry-printer/
├── Cargo.toml
├── README.md
└── src/
    ├── main.rs       # HTTP-сервер (axum), /print, /health; хардкод COM3 и 0.0.0.0:3001
    ├── printer.rs    # открытие COM-порта (serialport), запись ESC/POS-буфера
    ├── escpos.rs     # render_receipt: текст, CP866, заголовок, cut/eject
    └── qr.rs         # render_qr_escpos: QR как растр (GS v 0), центрирование на 576 точках
```

## Лицензия

Внутренний сервис проекта `laundry-stats`.
# laundry-printer

HTTP-сервис печати чеков для `laundry-stats`. Принимает `POST /print` с данными заказа, печатает чек на ESC/POS-принтере (USB или сетевой) и возвращает результат.

Сервис запускается **на хосте с принтером** (Windows или Linux), `laundry-stats` обращается к нему по HTTP. COM-порт не открывается — принтер подключается через USB (Windows: `usbprint.sys`, Linux: `/dev/usb/lp0`) или по сети (TCP `9100`).

**Работает без настройки.** Все параметры имеют значения по умолчанию, захардкоженные в коде. Переменные окружения нужны только чтобы переопределить дефолты (другой принтер, порт, кодировка, имя компании).

## Требования

- Rust 1.75+ (для сборки) или готовый бинарник
- Принтер ESC/POS с поддержкой кириллицы (или драйвер, который её добавляет)
- Свободный порт `3001` (по умолчанию)
- Для USB (Windows): установленный драйвер принтера
- Для USB (Linux): доступ к `/dev/usb/lp0` (пользователь в группе `lp`)
- Для сетевого принтера: принтер с поддержкой Raw TCP (порт `9100`)

## Быстрый старт (без env)

```bash
cargo build --release
./target/release/laundry-printer
```

Сервис слушает `0.0.0.0:3001`, печатает на USB-принтер, который определён автоматически (Windows: принтер по умолчанию; Linux: первый `/dev/usb/lp*`), кириллица в `cp866`, ширина `48`, имя компании `Laundry`, QR включён.

Проверка:

```bash
curl http://localhost:3001/health
# {"ok":true,"printer_ready":true}
```

## Переменные окружения (опционально)

Все переменные **необязательны**. Значения по умолчанию захардкожены в коде (`config.rs`). env нужен только для переопределения.

| Переменная | Значение по умолчанию (в коде) | Описание |
|---|---|---|
| `LAUNDRY_PRINTER_PORT` | `3001` | HTTP-порт сервиса |
| `LAUNDRY_PRINTER_ADDR` | `0.0.0.0` | Адрес, на котором слушает сервис |
| `PRINTER_KIND` | `usb` | `usb` / `network` / `file` (для отладки) |
| `PRINTER_USB_PATH` | `auto` | Windows: имя принтера; Linux: `/dev/usb/lp0`. `auto` — определить автоматически |
| `PRINTER_NETWORK_HOST` | — | IP сетевого принтера (только при `PRINTER_KIND=network`; без него сетевой режим не включится) |
| `PRINTER_NETWORK_PORT` | `9100` | Порт сетевого принтера |
| `PRINTER_FILE_PATH` | `/tmp/receipt.bin` | Путь для `PRINTER_KIND=file` (отладка без принтера) |
| `PRINTER_WIDTH` | `48` | Ширина чека в символах (`48` для 80 мм, `32` для 58 мм) |
| `PRINTER_CHARSET` | `cp866` | Кодировка кириллицы (`cp866` / `cp1251` / `utf8`) |
| `COMPANY_NAME` | `Laundry` | Название компании на чеке |
| `PRINTER_LOG` | `info` | Уровень логов (`RUST_LOG`) |
| `PRINTER_QR_ENABLED` | `true` | Печатать QR-код с `qr_data` |

Пример переопределения (только если нужно):

```bash
LAUNDRY_PRINTER_PORT=3002 COMPANY_NAME="Прачечная" ./laundry-printer
```

## Запуск

### Windows (USB-принтер, значения по умолчанию)

```powershell
.\laundry-printer.exe
```

Переопределение (например, другой порт и имя принтера):

```powershell
$env:LAUNDRY_PRINTER_PORT = "3002"
$env:PRINTER_USB_PATH = "POS-80"
$env:COMPANY_NAME = "Прачечная"
.\laundry-printer.exe
```

### Linux (USB-принтер, значения по умолчанию)

```bash
./laundry-printer
```

Переопределение:

```bash
LAUNDRY_PRINTER_PORT=3002 PRINTER_USB_PATH=/dev/usb/lp0 COMPANY_NAME="Прачечная" ./laundry-printer
```

### Сетевой принтер

Сетевой режим требует `PRINTER_KIND=network` и `PRINTER_NETWORK_HOST` (дефолта для хоста нет):

```bash
PRINTER_KIND=network PRINTER_NETWORK_HOST=192.168.2.50 ./laundry-printer
```

### Отладка без принтера

```bash
PRINTER_KIND=file ./laundry-printer
# POST /print запишет ESC/POS-команды в /tmp/receipt.bin
```

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
  "amount_rub": 400,
  "payment_method": "card",
  "paid_at": "2026-11-13T18:42:05+03:00",
  "company_name": "Прачечная",
  "qr_data": "ORDER:550e8400-e29b-41d4-a716-446655440000"
}
```

| Поле | Тип | Обязательно | Описание |
|---|---|---|---|
| `order_id` | string (UUID) | да | ID заказа |
| `machine_id` | string | да | ID машины (`w1`, `w6`, ...) |
| `program` | string | да | **Название** программы (label), а не id |
| `duration_min` | int | да | Длительность, минуты |
| `amount_rub` | int | да | Сумма, ₽ |
| `payment_method` | string | да | `card` / `cash` |
| `paid_at` | string (RFC3339) | да | Время оплаты в `RECEIPT_TZ` (`laundry-stats`), с offset |
| `company_name` | string | нет | Название компании (fallback — `COMPANY_NAME` из кода/env) |
| `qr_data` | string | нет | Данные для QR-кода |

**Response `200`:**

```json
{ "ok": true }
```

**Response `4xx`/`5xx`:**

```json
{ "error": "printer not ready" }
```

| Код | Причина |
|---|---|
| `400` | Некорректный JSON / отсутствует обязательное поле |
| `503` | Принтер не готов (offline, нет бумаги, занят) |
| `500` | Ошибка записи в принтер |

`laundry-stats` при `500`/`503` логирует `warn` и **не** ломает оплату: заказ остаётся `paid`, киоск показывает ошибку печати и кнопку повтора.

### `GET /health`

Проверка живости сервиса.

```json
{ "ok": true, "printer_ready": true }
```

`printer_ready = false`, если принтер offline / нет бумаги.

### `GET /metrics` (опционально)

Prometheus-метрики: `printer_requests_total`, `printer_errors_total`, `printer_print_duration_seconds`.

## Формат чека

Пример (80 мм, 48 символов):

```
================================================
                ПРАЧЕЧНАЯ
================================================
Заказ:  550e8400-e29b-41d4-a716-446655440000
Машина: w6
Программа: Умеренное загрязнение
Длительность: 45 мин
Сумма: 400 ₽
Оплата: картой
Время: 13.11.2026 18:42:05
================================================
        Спасибо за использование!
================================================
[QR: ORDER:550e8400-...]
```

Правила:

- `program` печатается **как есть** (label из `programs`), не id.
- `paid_at` парсится как RFC3339 с offset и печатается **в том же поясе**, что пришёл (`+03:00` → МСК). **Не конвертируйте** повторно в свою TZ — иначе двойной сдвиг.
- Кириллица кодируется в `PRINTER_CHARSET` (`cp866` по умолчанию).
- `qr_data` печатается QR-кодом, если `PRINTER_QR_ENABLED=true` и принтер поддерживает ESC/POS QR (`GS ( k`).

## Принтеры и совместимость

| Принтер | Подключение | Замечания |
|---|---|---|
| POS-80 (USB) | Windows: имя принтера; Linux: `/dev/usb/lp0` | `PRINTER_WIDTH=48`, `cp866` |
| Xprinter XP-58 | USB | `PRINTER_WIDTH=32`, `cp866` |
| Epson TM-T20 | USB / network | `cp866`, сетевой — порт `9100` |
| Generic ESC/POS | network | Raw TCP `9100` |

Если принтер не поддерживает кириллицу в `cp866` — попробуйте `cp1251` или `utf8` (зависит от прошивки). Проверить: `PRINTER_KIND=file`, отправить на принтер вручную (`cat /tmp/receipt.bin > /dev/usb/lp0` или через `raw` на Windows).

## Запуск как служба

### Windows (NSSM)

```powershell
nssm install laundry-printer "C:\laundry\laundry-printer.exe"
nssm start laundry-printer
```

Без env — сервис использует дефолты. Если нужны переопределения:

```powershell
nssm set laundry-printer AppEnvironmentExtra `
  LAUNDRY_PRINTER_PORT=3002 `
  PRINTER_USB_PATH=POS-80 `
  COMPANY_NAME=Прачечная
```

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

[Install]
WantedBy=multi-user.target
```

Без `Environment=` — сервис использует дефолты. Переопределения добавляются строками `Environment=...`.

```bash
sudo systemctl enable --now laundry-printer
sudo systemctl status laundry-printer
```

## Интеграция с `laundry-stats`

В `.env` `laundry-stats`:

```env
LAUNDRY_PRINTER_URL=http://laundry-printer:3001
COMPANY_NAME=Прачечная
RECEIPT_TZ=Europe/Moscow
```

Если `laundry-printer` запущен на другом хосте (не в Docker-сети), используйте IP: `http://192.168.2.143:3001`.

Проверка из контейнера `app`:

```powershell
docker compose exec app curl -s http://192.168.2.143:3001/health
# {"ok":true,"printer_ready":true}
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
    "amount_rub": 400,
    "payment_method": "card",
    "paid_at": "2026-11-13T18:42:05+03:00",
    "company_name": "Прачечная",
    "qr_data": "ORDER:test-0001"
  }'
```

## Troubleshooting

| Симптом | Причина | Решение |
|---|---|---|
| `printer not ready` (`503`) | Принтер offline, нет бумаги, неверный `PRINTER_USB_PATH` | Проверить принтер; `PRINTER_KIND=file` для отладки |
| `Permission denied` (Linux) | Нет доступа к `/dev/usb/lp0` | `sudo usermod -aG lp $USER`, перелогиниться |
| Кириллица печатается `????` | Неверная кодировка | `PRINTER_CHARSET=cp1251` или `utf8`; проверить прошивку принтера |
| Чек «обрезан» справа | `PRINTER_WIDTH` больше, чем у принтера | `PRINTER_WIDTH=32` для 58 мм, `48` для 80 мм |
| Двойной сдвиг времени на чеке | Сервис конвертирует `paid_at` в свою TZ | Печатать `paid_at` **как есть** (он уже в `RECEIPT_TZ` от `app`) |
| `program` = `medium` вместо названия | `laundry-stats` передал id | Проверить `program_label` в `app`; в `POST /print` должен быть label |
| QR не печатается | Принтер не поддерживает ESC/POS QR | `PRINTER_QR_ENABLED=false` или обновить прошивку |
| `connection refused` из `app` | `LAUNDRY_PRINTER_URL` неверный / сервис не запущен | `curl.exe http://<host>:3001/health` с хоста `app` |
| `500` на `POST /print` | Ошибка записи в принтер | Логи `laundry-printer` (`PRINTER_LOG=debug`), `PRINTER_KIND=file` для проверки формата |

## Метрики (опционально)

| Метрика | Тип | Описание |
|---|---|---|
| `printer_requests_total` | counter | Всего запросов `POST /print` |
| `printer_errors_total` | counter | Ошибки печати |
| `printer_print_duration_seconds` | histogram | Время печати |
| `printer_ready` | gauge | 1 = принтер готов |

## Лицензия

Внутренний сервис проекта `laundry-stats`.
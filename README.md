# VietCalendar (Rust)

Welcome to the Rust port of the **VietCalendar** API! 🎉 

This project was successfully migrated from its original Java 11 / Maven Vert.x codebase to a high-performance, containerized Rust web application built on top of **Axum** and **Tokio**.

---

## 🚀 API Endpoints

The server exposes the following RESTful endpoints (default port: `8080` or configurable via `$PORT` / `$HTTP_PORT`):

### 1. Today's Lunar Date
* **Endpoint:** `GET /`
* **Description:** Returns the current lunar date anchored to Vietnam Indochina Time (UTC+7).
* **Response (`200 OK`):**
  ```json
  {
    "dd": 17,
    "mm": 7,
    "yyyy": 2026
  }
  ```

---

### 2. Convert Solar to Lunar (Query Parameters)
* **Endpoint:** `GET /lunar`
* **Query Parameters:**
  | Parameter | Type | Required | Default | Description |
  | :--- | :---: | :---: | :---: | :--- |
  | `dd` | integer | Yes | - | Solar day (1–31) |
  | `mm` / `MM` | integer | Yes | - | Solar month (1–12) |
  | `yyyy` | integer | Yes | - | Solar year |
  | `timezone` / `timeZone` | float | No | `7.0` | Timezone offset (default: Vietnam UTC+7) |
* **Example Request:**
  ```http
  GET /lunar?dd=12&mm=9&yyyy=2015
  ```
* **Response (`200 OK`):**
  ```json
  {
    "dd": 30,
    "mm": 7,
    "yyyy": 2015
  }
  ```
* **Error Response (`400 Bad Request`):**
  ```json
  {
    "error": "Invalid solar date: 31/02/2024"
  }
  ```

---

### 3. Convert Solar to Lunar (Path Parameter)
* **Endpoint:** `GET /lunar/{ddMMyyyy}`
* **Path Parameter:**
  * `ddMMyyyy` (8-digit string): e.g. `12092015` for 12 September 2015.
* **Example Request:**
  ```http
  GET /lunar/12092015
  ```
* **Response (`200 OK`):**
  ```json
  {
    "dd": 30,
    "mm": 7,
    "yyyy": 2015
  }
  ```

---

### 4. Check Vietnam Public Holiday
* **Endpoint:** `GET /vietnam-holiday`
* **Description:** Determines if a date is an official Vietnam holiday (including fixed solar holidays, Giỗ Tổ Hùng Vương, Tết Nguyên Đán Eve + Days 1–3, weekends, and weekend compensatory leave / nghỉ bù).
* **Query Parameters:** `dd`, `mm` (or `MM`), `yyyy`.
* **Example Request:**
  ```http
  GET /vietnam-holiday?dd=30&mm=4&yyyy=2024
  ```
* **Response (`200 OK`):**
  ```json
  true
  ```

---

### 5. Interactive Swagger / OpenAPI Documentation
* **Swagger UI:** `http://localhost:8080/swagger-ui`
* **OpenAPI 3.0 Spec:** `http://localhost:8080/api-docs/openapi.json`

---

## 🛠️ Running Locally & Testing

```bash
# Run unit & integration tests
cargo test

# Run the server locally on http://localhost:8080
cargo run

# Build production Docker container
docker build -t vietcalendar-rs .
```

---

## 📖 Architecture & Design Decisions

For detailed architectural breakdown, domain modeling rationale, astronomical algorithms, and verification details, refer to:
* **[`docs/ARCHITECTURE_DECISIONS.md`](docs/ARCHITECTURE_DECISIONS.md)** (Architecture Decision Record)
* **[`GEMINI.md`](GEMINI.md)** (Antigravity AI repository guidelines)


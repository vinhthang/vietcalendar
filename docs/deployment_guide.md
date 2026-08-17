# VietCalendar Deployment Guide

This guide details how to deploy the **VietCalendar HTTP REST Web Server** and how to run the **MCP Stdio Server**.

---

## ☁️ 1. Google Cloud Run (Recommended Production Target)

Google Cloud Run is an ideal serverless container platform for Rust web services due to ultra-fast cold starts (<50ms) and automatic scale-to-zero pricing.

### Prerequisites
* Google Cloud SDK (`gcloud` CLI) installed and authenticated.
* An active GCP Project ID.

### Steps

```bash
# 1. Build and push container to Google Artifact Registry (or Container Registry)
gcloud builds submit --tag gcr.io/YOUR_PROJECT_ID/vietcalendar:latest

# 2. Deploy to Cloud Run
gcloud run deploy vietcalendar \
  --image gcr.io/YOUR_PROJECT_ID/vietcalendar:latest \
  --platform managed \
  --region asia-southeast1 \
  --allow-unauthenticated \
  --memory 256Mi \
  --cpu 1 \
  --min-instances 0 \
  --max-instances 10
```

> [!NOTE]
> Cloud Run automatically injects the `$PORT` environment variable. Our Axum server reads `$PORT` dynamically at startup and binds to `0.0.0.0:$PORT`.

---

## 🚀 2. Fly.io Deployment

Fly.io runs applications close to users on global edge servers.

```bash
# 1. Install Fly CLI & login
fly auth login

# 2. Launch application (creates fly.toml)
fly launch --name vietcalendar --region sin

# 3. Deploy
fly deploy
```

---

## 🐳 3. Self-Hosted VPS (Docker / Docker Compose)

Deploy directly on any Ubuntu/Debian Linux virtual private server:

```bash
# Run container with auto-restart on port 8080
docker run -d \
  --name vietcalendar \
  -p 8080:8080 \
  --restart unless-stopped \
  vietcalendar-rs
```

### Docker Compose (`docker-compose.yml`)

```yaml
version: '3.8'

services:
  vietcalendar:
    build: .
    ports:
      - "8080:8080"
    environment:
      - PORT=8080
      - RUST_LOG=info
    restart: unless-stopped
```

---

## 🤖 4. Running the Model Context Protocol (MCP) Server

The MCP server runs locally via standard I/O (`stdio`):

### Option A: Local Release Binary
```bash
cargo build --release --bin vietcalendar-mcp
./target/release/vietcalendar-mcp
```

### Option B: Docker Stdio
```bash
docker run -i --rm vietcalendar-rs vietcalendar mcp
```

# Performance & Load Testing Guide

This guide provides step-by-step instructions for load testing **VietCalendar** running inside Docker using modern containerized benchmarking tools.

---

## 1. Start the VietCalendar Docker Container

Before running load tests, start the server container:

```bash
docker run -d \
  --name vietcalendar \
  -p 8080:8080 \
  --rm \
  ghcr.io/vinhthang/vietcalendar:latest
```

---

## 2. Load Testing Tools (No Installation Needed)

### Option A: `bombardier` (Fastest, High Concurrency)

[Bombardier](https://github.com/codesenberg/bombardier) is a fast HTTP benchmarking tool.

#### Test 1: Solar to Lunar Conversion Endpoint
```bash
docker run --rm alpine/bombardier \
  -c 100 \
  -d 10s \
  http://host.docker.internal:8080/convert/solar-to-lunar/2024-02-10
```

#### Test 2: Holiday & Compensatory Engine
```bash
docker run --rm alpine/bombardier \
  -c 100 \
  -d 10s \
  "http://host.docker.internal:8080/vietnam-holiday?dd=30&mm=4&yyyy=2024"
```

#### Test 3: Today's Calendar Snapshot
```bash
docker run --rm alpine/bombardier \
  -c 100 \
  -d 10s \
  http://host.docker.internal:8080/
```

* **Parameters:**
  * `-c 100`: 100 concurrent HTTP connections.
  * `-d 10s`: 10-second duration test.
  * `http://host.docker.internal:8080`: Routes from within the benchmark container to your local Docker host.

---

### Option B: `grafana/k6` (Advanced Metrics & Percentiles)

[k6](https://k6.io/) provides granular latency distributions (p50, p90, p95, p99) and ramp-up scenarios.

#### Run a 3-Stage Ramp-up Test:
```bash
docker run --rm -i grafana/k6 run - << 'EOF'
import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  stages: [
    { duration: '5s', target: 50 },   // Ramp-up to 50 users
    { duration: '15s', target: 200 }, // Stress test at 200 concurrent users
    { duration: '5s', target: 0 },    // Ramp-down
  ],
  thresholds: {
    http_req_duration: ['p(95)<10'],  // 95% of requests must complete within 10ms
    http_req_failed: ['rate<0.01'],   // Error rate under 1%
  },
};

export default function () {
  const res = http.get('http://host.docker.internal:8080/convert/solar-to-lunar/2024-02-10');
  check(res, { 'status is 200': (r) => r.status === 200 });
}
EOF
```

---

### Option C: `williamyeh/wrk` (Classic Multi-Threaded Benchmark)

```bash
docker run --rm williamyeh/wrk \
  -t 4 \
  -c 100 \
  -d 10s \
  http://host.docker.internal:8080/convert/solar-to-lunar/2024-02-10
```

---

## 3. Stop the Server Container

```bash
docker stop vietcalendar
```

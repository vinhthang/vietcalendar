import http from 'k6/http';
import { check, group, sleep } from 'k6';

// k6 Options: 30-second multi-stage load test with concurrency up to 100 users
export const options = {
  stages: [
    { duration: '5s', target: 20 },   // Warm-up to 20 virtual users (VUs)
    { duration: '15s', target: 100 }, // Stress test at 100 concurrent VUs
    { duration: '5s', target: 0 },    // Cool down
  ],
  thresholds: {
    http_req_duration: ['p(95)<10', 'p(99)<25'], // 95% of requests < 10ms, 99% < 25ms
    http_req_failed: ['rate<0.01'],              // Error rate must be under 1%
  },
};

const BASE_URL = 'http://host.docker.internal:8080';

// Helper: Generate random integer between min and max (inclusive)
function randomInt(min, max) {
  return Math.floor(Math.random() * (max - min + 1)) + min;
}

// Helper: Generate a random valid ISO date between 1900 and 2050
function getRandomDate() {
  const year = randomInt(1900, 2050);
  const month = randomInt(1, 12);
  const maxDay = (month === 2) ? 28 : ([4, 6, 9, 11].includes(month) ? 30 : 31);
  const day = randomInt(1, maxDay);

  const mm = String(month).padStart(2, '0');
  const dd = String(day).padStart(2, '0');
  return { year, month, day, iso: `${year}-${mm}-${dd}` };
}

export default function () {
  const d = getRandomDate();
  const randSelector = Math.random();

  // Traffic Distribution across 5 Endpoints:
  if (randSelector < 0.30) {
    // 30% Traffic: Dynamic Solar to Lunar (Path Param)
    const res = http.get(`${BASE_URL}/convert/solar-to-lunar/${d.iso}`, {
      tags: { name: 'GET /convert/solar-to-lunar/:date' },
    });
    check(res, {
      'solar-to-lunar status is 200': (r) => r.status === 200,
      'solar-to-lunar has lunar dd': (r) => JSON.parse(r.body).dd !== undefined,
    });
  } else if (randSelector < 0.55) {
    // 25% Traffic: Dynamic Lunar to Solar (Path Param)
    const res = http.get(`${BASE_URL}/convert/lunar-to-solar/${d.iso}`, {
      tags: { name: 'GET /convert/lunar-to-solar/:date' },
    });
    check(res, {
      'lunar-to-solar valid response': (r) => r.status === 200 || r.status === 400,
    });
  } else if (randSelector < 0.75) {
    // 20% Traffic: Dynamic Vietnam Holiday & Compensatory Check
    const res = http.get(`${BASE_URL}/vietnam-holiday?dd=${d.day}&mm=${d.month}&yyyy=${d.year}`, {
      tags: { name: 'GET /vietnam-holiday' },
    });
    check(res, {
      'holiday check status is 200': (r) => r.status === 200,
      'holiday check returns boolean': (r) => typeof JSON.parse(r.body) === 'boolean',
    });
  } else if (randSelector < 0.90) {
    // 15% Traffic: Dynamic Solar to Lunar (Query Params)
    const res = http.get(`${BASE_URL}/lunar?dd=${d.day}&mm=${d.month}&yyyy=${d.year}`, {
      tags: { name: 'GET /lunar (Query Params)' },
    });
    check(res, {
      'lunar query status is 200': (r) => r.status === 200,
      'lunar query has dd': (r) => JSON.parse(r.body).dd !== undefined,
    });


  } else {
    // 10% Traffic: Home Endpoint (Current Day Anchor)
    const res = http.get(`${BASE_URL}/`, {
      tags: { name: 'GET / (Home - Today)' },
    });
    check(res, {
      'home status is 200': (r) => r.status === 200,
      'home has yyyy': (r) => JSON.parse(r.body).yyyy !== undefined,
    });
  }
}

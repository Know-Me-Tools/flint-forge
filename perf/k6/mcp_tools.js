import http from 'k6/http';
import { check } from 'k6';

export const options = {
  vus: 10,
  duration: '30s',
  thresholds: { http_req_duration: ['p(99)<100'] },
};

const BASE = __ENV.BASE_URL || 'http://localhost:8080';
const TOKEN = __ENV.TOKEN || '';

export default function () {
  const params = {
    headers: {
      Authorization: `Bearer ${TOKEN}`,
      'Content-Type': 'application/json',
    },
  };
  const res = http.get(`${BASE}/mcp/v1/tools`, params);
  check(res, {
    'status 200': (r) => r.status === 200,
    'has tools': (r) => Array.isArray(r.json('tools')),
  });
}

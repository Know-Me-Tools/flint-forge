import http from 'k6/http';
import { check } from 'k6';

export const options = {
  vus: 10,
  duration: '30s',
  thresholds: { http_req_duration: ['p(99)<100'] },
};

const BASE = __ENV.BASE_URL || 'http://localhost:8080';
const TOKEN = __ENV.TOKEN || '';

const QUERY = `
  query ComponentsList {
    componentsCollection(first: 5) {
      edges {
        node {
          id
          slug
          category
        }
      }
    }
  }
`;

export default function () {
  const params = {
    headers: {
      Authorization: `Bearer ${TOKEN}`,
      'Content-Type': 'application/json',
    },
  };
  const res = http.post(`${BASE}/graphql`, JSON.stringify({ query: QUERY }), params);
  check(res, {
    'status 200': (r) => r.status === 200,
    'no errors': (r) => r.json('errors') === null || r.json('errors') === undefined,
    'has data': (r) => r.json('data') !== null,
  });
}

# p10-c001 — TLS Termination via Caddy

**Phase:** 10 — Production Launch
**Priority:** P0
**Depends on:** p10-c003 (compose changes should apply cleanly after wasmtime upgrade)

## Problem

`fdb-gateway` and `fke-server` expose raw HTTP on ports 8080/8090 with no TLS
layer. Accepting production traffic on plaintext HTTP is unacceptable for any
service handling JWTs and tenant data.

## Solution

Add a `caddy` service to `docker-compose.prod.yml` as a TLS-terminating reverse
proxy. Caddy provides automatic Let's Encrypt certificate provisioning via the
ACME protocol — zero manual cert management.

### Caddyfile (mounted as volume)

```
{$FLINT_DOMAIN} {
    reverse_proxy fdb-gateway:8080
    tls {$CADDY_TLS_EMAIL}
}

kiln.{$FLINT_DOMAIN} {
    reverse_proxy fke-server:8090
    tls {$CADDY_TLS_EMAIL}
}
```

Place at `docker/caddy/Caddyfile`.

### `docker-compose.prod.yml` additions

```yaml
services:
  caddy:
    image: caddy:2-alpine
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./docker/caddy/Caddyfile:/etc/caddy/Caddyfile:ro
      - caddy_data:/data
      - caddy_config:/config
    environment:
      FLINT_DOMAIN: ${FLINT_DOMAIN}
      CADDY_TLS_EMAIL: ${CADDY_TLS_EMAIL}
    depends_on:
      - fdb-gateway
      - fke-server

  fdb-gateway:
    ports: !reset []         # remove 8080 from public interface

  fke-server:
    ports: !reset []         # remove 8090 from public interface

volumes:
  caddy_data:
  caddy_config:
```

### `.env.example` additions

```bash
# ── TLS (production) ─────────────────────────────────────────────────────────
# Public domain name for fdb-gateway (required for Let's Encrypt).
FLINT_DOMAIN=forge.example.com
# Email for Let's Encrypt ACME account.
CADDY_TLS_EMAIL=ops@example.com
```

### Runbook update

Add §10 to `docs/runbook.md` covering:
- First-run TLS provisioning flow (Caddy contacts Let's Encrypt on startup)
- Cert renewal (automatic; Caddy renews 30 days before expiry)
- Troubleshooting cert failures (`docker compose logs caddy`)
- `version: '3.9'` removal from all compose files (cosmetic debt from p9)

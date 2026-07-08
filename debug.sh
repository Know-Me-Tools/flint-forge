#!/usr/bin/env bash
set -e
docker rm -f flint-debug > /dev/null 2>&1 || true
docker run -d --name flint-debug --entrypoint bash flint-forge-pg:18 -c 'while true; do sleep 3600; done'
sleep 2
docker cp /Users/gqadonis/Projects/prometheus/flint-forge/debug_query.sql flint-debug:/tmp/flint_query.sql
docker exec flint-debug bash -c 'su postgres -c "initdb -D /tmp/pgdata --auth-local=trust --auth-host=trust"'
docker exec flint-debug bash -c 'su postgres -c "pg_ctl -D /tmp/pgdata -o \"-p 5433 -c shared_preload_libraries=pg_net,pg_cron,flint_llm\" start"'
sleep 3
docker exec flint-debug bash -c 'su postgres -c "psql -p 5433 -U postgres -f /tmp/flint_query.sql"'
docker stop flint-debug > /dev/null
docker rm flint-debug > /dev/null

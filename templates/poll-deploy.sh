#!/usr/bin/env bash
# Target path on server: /opt/walrus/poll-deploy.sh
set -euo pipefail

APP_DIR="${APP_DIR:-/opt/walrus}"
WALRUS_TAG="${WALRUS_TAG:-latest}"
LOCK_FILE="${LOCK_FILE:-/tmp/walrus-poll-deploy.lock}"
COMPOSE_FILE="${COMPOSE_FILE:-$APP_DIR/docker-compose.yml}"
SERVICE_NAME="${SERVICE_NAME:-walrus-server}"
HEALTH_URL="${HEALTH_URL:-http://127.0.0.1:3000/health}"

if command -v flock >/dev/null 2>&1; then
  exec 9>"${LOCK_FILE}"
  if ! flock -n 9; then
    echo "poll deploy already running, skipping"
    exit 0
  fi
fi

if [[ ! -f "${COMPOSE_FILE}" ]]; then
  echo "compose file not found: ${COMPOSE_FILE}" >&2
  exit 1
fi

cd "${APP_DIR}"

docker compose -f "${COMPOSE_FILE}" config >/dev/null
docker compose -f "${COMPOSE_FILE}" pull "${SERVICE_NAME}"
docker compose -f "${COMPOSE_FILE}" up -d --remove-orphans

for _ in $(seq 1 30); do
  status_code="$(curl -s -o /dev/null -w '%{http_code}' "${HEALTH_URL}" || true)"
  if [[ "${status_code}" == "200" ]]; then
    echo "deployment successful for WALRUS_TAG=${WALRUS_TAG}, health=${status_code}"
    docker compose -f "${COMPOSE_FILE}" ps
    exit 0
  fi
  sleep 2
done

echo "health check failed after deployment, recent logs:"
docker compose -f "${COMPOSE_FILE}" logs --tail=200 "${SERVICE_NAME}" || true
exit 1

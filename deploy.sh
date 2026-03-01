#!/usr/bin/env bash
set -euo pipefail

APP_DIR="${APP_DIR:-/opt/walrus}"
COMPOSE_FILE="${COMPOSE_FILE:-$APP_DIR/docker-compose.yml}"
SERVICE_NAME="${SERVICE_NAME:-walrus-server}"
HEALTH_URL="${HEALTH_URL:-http://127.0.0.1:3000/auth/whoami}"

if [[ $# -gt 0 ]]; then
  export WALRUS_TAG="$1"
fi

if [[ -z "${WALRUS_TAG:-}" ]]; then
  echo "WALRUS_TAG is required (pass as first argument or environment variable)." >&2
  exit 1
fi

if [[ -n "${GHCR_USER:-}" && -n "${GHCR_TOKEN:-}" ]]; then
  echo "${GHCR_TOKEN}" | docker login ghcr.io -u "${GHCR_USER}" --password-stdin
fi

cd "${APP_DIR}"

if [[ ! -f "${COMPOSE_FILE}" ]]; then
  echo "Compose file not found: ${COMPOSE_FILE}" >&2
  exit 1
fi

docker compose -f "${COMPOSE_FILE}" config >/dev/null
docker compose -f "${COMPOSE_FILE}" pull "${SERVICE_NAME}"
docker compose -f "${COMPOSE_FILE}" up -d --remove-orphans

for _ in $(seq 1 30); do
  status_code="$(curl -s -o /dev/null -w '%{http_code}' "${HEALTH_URL}" || true)"
  if [[ "${status_code}" == "200" || "${status_code}" == "401" ]]; then
    echo "Deployment successful. Health endpoint returned ${status_code}."
    docker compose -f "${COMPOSE_FILE}" ps
    exit 0
  fi
  sleep 2
done

echo "Health check failed after deployment. Recent logs:"
docker compose -f "${COMPOSE_FILE}" logs --tail=200 "${SERVICE_NAME}" || true
exit 1

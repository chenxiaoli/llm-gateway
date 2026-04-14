#!/bin/bash

# LLM Gateway Dev Deploy Script
# Usage: ./run.sh

set -e

IMAGE_NAME="${IMAGE_NAME:-ghcr.io/chenxiaoli/llm-gateway}"
CONTAINER_NAME="llm-gateway-dev"
PORT=3080
CONFIG_PATH="/opt/dev/llm-gateway/config.toml"
DATA_PATH="/opt/dev/llm-gateway/data"

echo "=== Pulling image ==="
docker pull "$IMAGE_NAME:develop"

echo "=== Stopping existing container ==="
docker stop "$CONTAINER_NAME" 2>/dev/null || true
docker rm "$CONTAINER_NAME" 2>/dev/null || true

echo "=== Starting new container ==="
docker run -d \
  --name "$CONTAINER_NAME" \
  --restart unless-stopped \
  -p "${PORT}:8080" \
  -v "${CONFIG_PATH}:/app/config.toml:ro" \
  -v "${DATA_PATH}:/app/data" \
  "$IMAGE_NAME:develop"

echo "=== Waiting for container to start ==="
sleep 3

echo "=== Checking container status ==="
STATUS=$(docker inspect --format='{{.State.Status}}' "$CONTAINER_NAME" 2>/dev/null)
if [ "$STATUS" != "running" ]; then
  echo "ERROR: container is '$STATUS', expected 'running'"
  docker logs "$CONTAINER_NAME" 2>&1 | tail -30
  exit 1
fi

echo "OK: $CONTAINER_NAME is running"
docker logs "$CONTAINER_NAME" 2>&1 | tail -5

echo "=== Cleaning up unused images ==="
docker image prune -f

echo "=== Done ==="
# Dev Deploy Setup

## GitHub Secrets (Repository Settings → Secrets and variables → Actions)

| Secret | Description |
|--------|-------------|
| `DEV_HOST` | Dev host IP or hostname (e.g. `192.168.1.100`) |
| `DEV_USER` | SSH username on dev host |
| `DEV_SSH_KEY` | Private SSH key (ed25519 recommended) |

## Dev Host Prerequisites

1. **Docker** installed and running
2. **SSH access** with the provided key
3. **Firewall**: port 8080 open

## First-run Setup on Dev Host

```bash
# Create directories
sudo mkdir -p /opt/llm-gateway
sudo chown $USER:$USER /opt/llm-gateway

# Create config.toml
cat > /opt/llm-gateway/config.toml << 'EOF'
[server]
host = "0.0.0.0"
port = 8080

[auth]
jwt_secret = "change-me-in-production"
allow_registration = true

[database]
driver = "sqlite"
sqlite_path = "./data/gateway.db"

[rate_limit]
flush_interval_secs = 30
window_size_secs = 60

[upstream]
timeout_secs = 30

[audit]
retention_days = 90
EOF

# Create data directory
mkdir -p /opt/llm-gateway/data
```

## How It Works

1. Push to `develop` triggers the workflow
2. Frontend tests + backend tests run
3. Docker image built and pushed to `ghcr.io/<owner>/llm-gateway:develop`
4. SSH deploys to dev host:
   - Pulls latest image
   - Stops and removes old container
   - Starts new container with volume mounts

## Verify Deployment

```bash
# On dev host
docker logs llm-gateway-dev

# From any machine
curl http://<DEV_HOST>:8080/version
```

## Manual Deploy

If you need to redeploy without a code change:

```bash
# In GitHub Actions → dev-deploy → Run workflow
# Or SSH manually:
ssh $DEV_USER@$DEV_HOST "docker pull ghcr.io/<owner>/llm-gateway:develop && docker restart llm-gateway-dev"
```

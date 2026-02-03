# Vantage Docker Environment

## File Structure

```
Vantage/
├── docker/
│   ├── Dockerfile.dev      # Development (full toolchain)
│   ├── Dockerfile.engine   # Production engine (minimal)
│   ├── Dockerfile.web      # Production frontend (nginx)
│   └── nginx.conf          # Nginx config for frontend
├── docker-compose.yml      # Development
├── docker-compose.prod.yml # Production
├── .dockerignore
├── git-config-docker.template
├── backend/                # Rust engine
└── frontend/               # Web UI
```

## Development

### First-time Setup

```bash
# 1. Create git config
cp git-config-docker.template git-config-docker
# Edit with your name/email

# 2. Build and start
docker compose build
docker compose up -d

# 3. Enter container
docker compose exec dev bash
```

### Daily Usage

```bash
# Start container
docker compose up -d

# Open terminal(s)
docker compose exec dev bash

# Stop
docker compose down
```

### Frontend Development

Inside the container:
```bash
cd /workspace/frontend
npm install
npm run dev  # Accessible at localhost:3000 or :5173
```

### Rust Development

Inside the container:
```bash
cd /workspace/backend
cargo build --release
cargo test
cargo watch -x "build --release"  # Auto-rebuild on changes
```

## Engine Testing

### SPRT Test (Quick Regression Check)

```bash
# Build both versions
cargo build --release
cp target/release/vantage /engines/candidates/vantage_new

# Run SPRT
cutechess-cli \
  -engine name=New cmd=/engines/candidates/vantage_new \
  -engine name=Old cmd=/engines/candidates/vantage_old \
  -each proto=uci tc=8+0.08 \
  -rounds 1000 \
  -concurrency 4 \
  -sprt elo0=0 elo1=5 alpha=0.05 beta=0.05 \
  -openings file=/openings/2moves_v2.pgn format=pgn \
  -pgnout /results/pgn/sprt.pgn
```

### Gauntlet (Full Rating Test)

```bash
cutechess-cli \
  -engine name=Vantage cmd=/engines/candidates/vantage \
  -engine name=Stockfish cmd=stockfish option.Hash=64 \
  -each proto=uci tc=40/60 \
  -rounds 100 \
  -concurrency 2 \
  -games 2 \
  -openings file=/openings/2moves_v2.pgn format=pgn \
  -pgnout /results/pgn/gauntlet.pgn

# Calculate ratings
ordo -p /results/pgn/gauntlet.pgn -a 2800 -A Stockfish
```

## Production

### Build Images

```bash
docker compose -f docker-compose.prod.yml build
```

### Run Locally

```bash
docker compose -f docker-compose.prod.yml up -d
# Frontend at http://localhost
```

### Deploy

```bash
# Tag and push to registry
docker tag vantage-web:latest your-registry/vantage-web:v1.0.0
docker tag vantage-engine:latest your-registry/vantage-engine:v1.0.0
docker push your-registry/vantage-web:v1.0.0
docker push your-registry/vantage-engine:v1.0.0
```

## Pre-installed Tools

| Tool | Purpose |
|------|---------|
| `cutechess-cli` | Run engine matches |
| `stockfish` | Reference engine |
| `ordo` | Calculate ELO ratings |
| `cargo-watch` | Auto-rebuild on file changes |
| `cargo-flamegraph` | Performance profiling |
| `python-chess` | Chess utilities |

## Volumes

| Volume | Contents | Persists After |
|--------|----------|----------------|
| `vantage-results` | PGN files, test database | `docker compose down` |
| `vantage-engines` | Compiled engine binaries | `docker compose down` |

Reset everything:
```bash
docker compose down -v
```

## Troubleshooting

### Permission Denied

```bash
# Inside container
chown -R root:root /workspace
```

### Port Already in Use

```bash
# Change port in docker-compose.yml or stop conflicting service
docker compose down
lsof -i :3000  # Find what's using the port
```

### Rebuild from Scratch

```bash
docker compose down -v
docker compose build --no-cache
docker compose up -d
```

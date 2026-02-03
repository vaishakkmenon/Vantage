# =============================================================================
# Vantage Chess Engine - Development & Testing Environment
# =============================================================================
# Supports: Development, Gauntlet Testing, SPRT, Distributed Workers
# =============================================================================

FROM ubuntu:24.04

# Prevent interactive prompts during install
ENV DEBIAN_FRONTEND=noninteractive

# -----------------------------------------------------------------------------
# 1. Core System Packages
# -----------------------------------------------------------------------------
RUN apt-get update && apt-get install -y --no-install-recommends \
    # Build essentials
    build-essential \
    cmake \
    pkg-config \
    # Version control
    git \
    # Utilities
    curl \
    wget \
    unzip \
    ca-certificates \
    # Editors
    vim \
    nano \
    # Debugging
    gdb \
    valgrind \
    # Python
    python3 \
    python3-venv \
    python3-pip \
    python3-dev \
    # Database (for test results)
    sqlite3 \
    libsqlite3-dev \
    # Networking (for distributed testing)
    openssh-client \
    rsync \
    netcat-openbsd \
    # Process management
    supervisor \
    && rm -rf /var/lib/apt/lists/*

# -----------------------------------------------------------------------------
# 2. Chess Testing Infrastructure
# -----------------------------------------------------------------------------

# Install Stockfish (anchor engine for Gauntlet)
RUN apt-get update \
    && apt-get install -y --no-install-recommends stockfish \
    && rm -rf /var/lib/apt/lists/* \
    && ln -sf /usr/games/stockfish /usr/local/bin/stockfish

# Install cutechess-cli dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    qt6-base-dev \
    libqt6core5compat6 \
    libfuse2 \
    && rm -rf /var/lib/apt/lists/*

# Install cutechess-cli (primary testing tool)
RUN cd /tmp \
    && wget -q https://github.com/cutechess/cutechess/releases/download/v1.4.0/Cute_Chess-1.4.0-x86_64.AppImage \
    && chmod +x Cute_Chess-1.4.0-x86_64.AppImage \
    && ./Cute_Chess-1.4.0-x86_64.AppImage --appimage-extract \
    && cp squashfs-root/usr/bin/cutechess-cli /usr/local/bin/ \
    && chmod +x /usr/local/bin/cutechess-cli \
    && rm -rf /tmp/Cute_Chess-1.4.0-x86_64.AppImage /tmp/squashfs-root

# Install Ordo (rating calculator for Gauntlet)
RUN cd /tmp \
    && git clone --depth 1 https://github.com/michiguel/Ordo.git \
    && cd Ordo \
    && make \
    && cp ordo /usr/local/bin/ \
    && rm -rf /tmp/Ordo

# -----------------------------------------------------------------------------
# 3. Node.js (for Claude Code CLI)
# -----------------------------------------------------------------------------
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && npm install -g npm@latest \
    && rm -rf /var/lib/apt/lists/*

# Install Claude Code CLI
RUN npm install -g @anthropic-ai/claude-code

# -----------------------------------------------------------------------------
# 4. Rust Toolchain
# -----------------------------------------------------------------------------
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain stable \
    && echo 'source /root/.cargo/env' >> /root/.bashrc

ENV PATH="/root/.cargo/bin:${PATH}"

# Install useful Rust tools
RUN cargo install cargo-watch cargo-flamegraph

# -----------------------------------------------------------------------------
# 5. Python Environment
# -----------------------------------------------------------------------------
RUN python3 -m venv /venv

ENV PATH="/venv/bin:${PATH}"
ENV VIRTUAL_ENV="/venv"

# Install Python packages for development and testing
RUN pip install --upgrade pip && pip install \
    # Development tools
    black \
    flake8 \
    mypy \
    pytest \
    pytest-timeout \
    # Data handling (for test results)
    pandas \
    sqlalchemy \
    # Scripting utilities
    click \
    rich \
    pyyaml \
    toml \
    # Chess utilities
    python-chess

# -----------------------------------------------------------------------------
# 6. Testing Infrastructure Directories
# -----------------------------------------------------------------------------
RUN mkdir -p /engines/anchors \
    && mkdir -p /engines/candidates \
    && mkdir -p /openings \
    && mkdir -p /results/pgn \
    && mkdir -p /results/db \
    && mkdir -p /queue

# Download a sample opening book (2moves for balanced testing)
RUN cd /openings \
    && wget -q https://raw.githubusercontent.com/official-stockfish/books/master/2moves_v2.pgn \      
    || echo "# Opening book download failed - add manually" > /openings/README.txt

# -----------------------------------------------------------------------------
# 7. Workspace Setup
# -----------------------------------------------------------------------------
WORKDIR /workspace
RUN mkdir -p /workspace

VOLUME ["/workspace"]

# -----------------------------------------------------------------------------
# 8. Environment Variables for Testing
# -----------------------------------------------------------------------------
ENV ENGINES_DIR="/engines"
ENV ANCHORS_DIR="/engines/anchors"
ENV CANDIDATES_DIR="/engines/candidates"
ENV OPENINGS_DIR="/openings"
ENV RESULTS_DIR="/results"
ENV QUEUE_DIR="/queue"

# -----------------------------------------------------------------------------
# 9. Default Entrypoint
# -----------------------------------------------------------------------------
ENTRYPOINT ["bash"]
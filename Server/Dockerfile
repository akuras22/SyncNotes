# Builds rnote-cli from source so the server can render a PDF preview for
# each synced .rnote file. rnote-cli isn't published anywhere as a binary
# (not on crates.io, no GitHub release artifacts), so it has to be compiled
# here. It has no GTK4/libadwaita dependency - just libcairo2 + librsvg2 -
# so it runs headless with no display server needed.

FROM rust:1-slim-trixie AS rnote-builder

# cairo-sys-rs (a dependency of rnote-engine) needs cairo >= 1.18, which
# Debian Bookworm's apt repos don't have (only 1.16) - Trixie ships 1.18.4.
# Both build and runtime stages must be on the same Debian release so the
# binary built here links against the same library versions it'll find
# at runtime in the final image below.
RUN apt-get update && apt-get install -y --no-install-recommends \
    git ca-certificates pkg-config build-essential \
    libcairo2-dev librsvg2-dev \
    && rm -rf /var/lib/apt/lists/*

RUN git clone --branch v0.14.2 --depth 1 https://github.com/flxzt/rnote /tmp/rnote \
    && cd /tmp/rnote \
    && cargo build --release -p rnote-cli \
    && cp target/release/rnote-cli /usr/local/bin/rnote-cli \
    && rm -rf /tmp/rnote

FROM python:3.12-slim-trixie

RUN apt-get update && apt-get install -y --no-install-recommends \
    libcairo2 librsvg2-2 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=rnote-builder /usr/local/bin/rnote-cli /usr/local/bin/rnote-cli

WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt
COPY . .

CMD ["python", "app.py"]

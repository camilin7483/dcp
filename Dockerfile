# DCP Daemon — multistage Docker build
# Use for CI/testing, not for daily desktop use (need X11/Wayland)

FROM rust:1.85-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .

RUN cargo build --release --workspace && \
    cp target/release/dcpd /usr/local/bin/dcpd && \
    cp target/release/dcp /usr/local/bin/dcp

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/bin/dcpd /usr/local/bin/dcpd
COPY --from=builder /usr/local/bin/dcp /usr/local/bin/dcp

EXPOSE 9527

ENTRYPOINT ["dcpd"]
CMD ["--foreground"]

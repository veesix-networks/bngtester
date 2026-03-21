# Copyright The bngtester Authors
# Licensed under the GNU General Public License v3.0 or later.
# SPDX-License-Identifier: GPL-3.0-or-later

FROM rust:1.85-alpine AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin && \
    echo 'fn main() {}' > src/bin/client.rs && \
    echo 'fn main() {}' > src/bin/server.rs && \
    echo '' > src/lib.rs && \
    apk add --no-cache musl-dev && \
    cargo build --release 2>/dev/null || true && \
    rm -rf src
COPY src/ src/
RUN cargo build --release && \
    strip target/release/bngtester-client target/release/bngtester-server

FROM alpine:3.21

RUN apk add --no-cache \
    dhcpcd \
    ppp \
    ppp-pppoe \
    iputils \
    iproute2 \
    iperf3

COPY --from=builder /build/target/release/bngtester-client /usr/local/bin/bngtester-client
COPY --from=builder /build/target/release/bngtester-server /usr/local/bin/bngtester-server
COPY images/shared/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]

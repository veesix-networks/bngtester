# Copyright The bngtester Authors
# Licensed under the GNU General Public License v3.0 or later.
# SPDX-License-Identifier: GPL-3.0-or-later

FROM rust:1.85-alpine AS builder
WORKDIR /build
RUN apk add --no-cache musl-dev make
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
RUN cargo build --release

FROM ubuntu:22.04

RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    isc-dhcp-client \
    ppp \
    iputils-ping \
    iproute2 \
    iperf3 \
    curl \
    ca-certificates \
    netbase \
    tcpdump \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/bngtester-client /usr/local/bin/bngtester-client
COPY --from=builder /build/target/release/bngtester-server /usr/local/bin/bngtester-server
COPY images/shared/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]

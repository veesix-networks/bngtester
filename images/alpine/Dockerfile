# Copyright The bngtester Authors
# Licensed under the GNU General Public License v3.0 or later.
# SPDX-License-Identifier: GPL-3.0-or-later

FROM rust:1.85-alpine AS builder
WORKDIR /build
RUN apk add --no-cache musl-dev make
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
RUN cargo build --release

FROM alpine:3.21

RUN apk add --no-cache \
    dhcpcd \
    ppp \
    ppp-pppoe \
    iputils \
    iproute2 \
    iperf3 \
    tcpdump

COPY --from=builder /build/target/release/bngtester-client /usr/local/bin/bngtester-client
COPY --from=builder /build/target/release/bngtester-server /usr/local/bin/bngtester-server
COPY images/shared/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]

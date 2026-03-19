# Copyright The bngtester Authors
# Licensed under the GNU General Public License v3.0 or later.
# SPDX-License-Identifier: GPL-3.0-or-later

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
    && rm -rf /var/lib/apt/lists/*

COPY shared/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]

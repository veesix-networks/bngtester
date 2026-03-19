# Copyright The bngtester Authors
# Licensed under the GNU General Public License v3.0 or later.
# SPDX-License-Identifier: GPL-3.0-or-later

FROM alpine:3.21

RUN apk add --no-cache \
    dhcpcd \
    ppp \
    ppp-pppoe \
    iputils \
    iproute2 \
    iperf3

COPY shared/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]

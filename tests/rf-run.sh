#!/bin/bash
# Copyright The bngtester Authors
# Licensed under the GNU General Public License v3.0 or later.
# SPDX-License-Identifier: GPL-3.0-or-later

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUT_DIR="${SCRIPT_DIR}/out"

if [ -z "$1" ]; then
    echo "Usage: $0 <test-suite-path> [extra robot args]"
    echo "Example: $0 tests/01-entrypoint-validation/"
    echo "         $0 tests/02-vlan-modes/ --variable SUBSCRIBER_IMAGE:veesixnetworks/bngtester:debian-latest"
    exit 1
fi

SUITE_PATH="$1"
shift
EXTRA_ARGS=("$@")

# --- Preflight checks ---
if ! docker info > /dev/null 2>&1; then
    echo "ERROR: Docker is not running or not accessible"
    exit 1
fi

mkdir -p "${OUT_DIR}"

# --- Venv setup ---
if [ ! -d "${SCRIPT_DIR}/.venv" ]; then
    echo "Creating Python virtual environment..."
    python3 -m venv "${SCRIPT_DIR}/.venv"
    source "${SCRIPT_DIR}/.venv/bin/activate"
    pip install -q robotframework
else
    source "${SCRIPT_DIR}/.venv/bin/activate"
fi

# --- Log naming ---
get_logname() {
    path=$1
    filename=$(basename "$path")
    if [[ "$filename" == *.* ]]; then
        dirname=$(dirname "$path")
        basename_noext=$(basename "$path" | cut -d. -f1)
        echo "${dirname##*/}-${basename_noext}"
    else
        echo "${filename}"
    fi
}

LOG_NAME=$(get_logname "${SUITE_PATH}")

echo "Running test suite: ${SUITE_PATH}"
echo "Output directory: ${OUT_DIR}"

robot \
    --consolecolors on \
    -r none \
    -l "${OUT_DIR}/${LOG_NAME}-log" \
    --output "${OUT_DIR}/${LOG_NAME}-out.xml" \
    "${EXTRA_ARGS[@]}" \
    "${SUITE_PATH}"

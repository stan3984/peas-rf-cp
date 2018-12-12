#!/bin/bash

set -e

if [[ $# -ne 2 ]]; then
    echo "usage: $0 localip numofbots" >&2
    exit 1
fi

trap 'kill 0' EXIT

for i in $(seq 1 "$2"); do
    ./client -j omg.peas-room -u anna"$i" -t "${1}:12345" --bot --log all --log-stderr 2>&1 | tee anna"$i".log &
    sleep 2
done

wait

#!/bin/bash

export RUST_LOG="info,spacetraders_rs=debug"

while true
do
    cargo run --bin run --release >> run.log 2>&1
    echo "Process crashed with exit code $?.  Respawning.." >&2
    echo "Process crashed with exit code $?.  Respawning.." >> run.log
    sleep 5
done

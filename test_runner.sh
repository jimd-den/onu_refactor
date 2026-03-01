#!/bin/bash
cargo build
for i in samples/*.onu; do
    stem=$(basename $i .onu)
    if [[ "$stem" == "guess" || "$stem" == "illegal_shared" ]]; then
        continue
    fi
    ./target/debug/onu_refactor $i > /dev/null 2>&1
    if [[ "$stem" == "echo_demo" ]]; then
        out=$(./${stem}_bin hello 2>/dev/null)
    else
        out=$(./${stem}_bin 2>/dev/null)
    fi
    echo "--- $stem ---"
    echo "$out"
    echo "--------------"
done

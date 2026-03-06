#!/bin/bash
cargo run --release -- samples/fib_naive_only.onu
./fib_naive_only_bin > output.txt
cat output.txt
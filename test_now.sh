#!/bin/bash
cd "$(dirname "$0")"
cargo run --quiet --example test_scheduler -- simple-test.yml

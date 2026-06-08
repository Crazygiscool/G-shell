#!/bin/sh
set -e

MODE="${1:---dev}"

case "$MODE" in
  --dev)
    shift 2>/dev/null || true
    cargo build 2>&1
    exec target/debug/g-shell "$@"
    ;;
  --bin)
    shift 2>/dev/null || true
    cargo build --release 2>&1
    exec target/release/g-shell "$@"
    ;;
  *)
    echo "Usage: $0 [--dev | --bin] [args...]" >&2
    echo "  --dev    Build debug (default) and run from target/debug/" >&2
    echo "  --bin    Build release to target/release/ and run" >&2
    exit 1
    ;;
esac

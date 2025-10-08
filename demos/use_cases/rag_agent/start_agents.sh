#!/bin/bash
set -e

WAIT_FOR_PIDS=()

log() {
  timestamp=$(python3 -c 'from datetime import datetime; print(datetime.now().strftime("%Y-%m-%d %H:%M:%S,%f")[:23])')
  message="$*"
  echo "$timestamp - $message"
}

cleanup() {
    log "Caught signal, terminating all user processes ..."
    for PID in "${WAIT_FOR_PIDS[@]}"; do
        if kill $PID 2> /dev/null; then
            log "killed process: $PID"
        fi
    done
    exit 1
}

trap cleanup EXIT

log "Starting query_parser agent on port 10500..."
uv run python -m rag_agent --rest-server --host 0.0.0.0 --rest-port 10500 --agent query_parser &
WAIT_FOR_PIDS+=($!)

log "Starting content_builder agent on port 10501..."
uv run python -m rag_agent --rest-server --host 0.0.0.0 --rest-port 10501 --agent content_builder &
WAIT_FOR_PIDS+=($!)

log "Starting response_generator agent on port 10502..."
uv run python -m rag_agent --rest-server --host 0.0.0.0 --rest-port 10502 --agent response_generator &
WAIT_FOR_PIDS+=($!)

for PID in "${WAIT_FOR_PIDS[@]}"; do
    wait "$PID"
done

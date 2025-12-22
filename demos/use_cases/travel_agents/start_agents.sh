#!/bin/bash
set -e

WAIT_FOR_PIDS=()

log() {
  timestamp=$(python3 -c 'from datetime import datetime; print(datetime.now().strftime("%Y-%m-%d %H:%M:%S,%f")[:23])')
  message="$*"
  echo "$timestamp - $message"
}

cleanup() {
    log "Caught signal, terminating all agent processes ..."
    for PID in "${WAIT_FOR_PIDS[@]}"; do
        if kill $PID 2> /dev/null; then
            log "killed process: $PID"
        fi
    done
    exit 1
}

trap cleanup EXIT

log "Starting weather agent on port 10510..."
uv run python -m travel_agents --host 0.0.0.0 --port 10510 --agent weather &
WAIT_FOR_PIDS+=($!)

log "Starting flight agent on port 10520..."
uv run python -m travel_agents --host 0.0.0.0 --port 10520 --agent flight &
WAIT_FOR_PIDS+=($!)

log "Starting currency agent on port 10530..."
uv run python -m travel_agents --host 0.0.0.0 --port 10530 --agent currency &
WAIT_FOR_PIDS+=($!)

log "All agents started successfully!"
log "  - Weather Agent: http://localhost:10510"
log "  - Flight Agent: http://localhost:10520"
log "  - Currency Agent: http://localhost:10530"
log ""
log "Waiting for agents to run..."

for PID in "${WAIT_FOR_PIDS[@]}"; do
    wait "$PID"
done

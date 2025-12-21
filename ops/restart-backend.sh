#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="/srv/movie-games-server"
BIN_PATH="${ROOT_DIR}/server"
LOG_PATH="${ROOT_DIR}/server.out.log"
PID_PATH="${ROOT_DIR}/server.pid"

mkdir -p "${ROOT_DIR}"

if [ ! -x "${BIN_PATH}" ]; then
  echo "binary not found or not executable: ${BIN_PATH}" >&2
  exit 1
fi

read_pid() {
  if [ -f "${PID_PATH}" ]; then
    cat "${PID_PATH}" 2>/dev/null || true
  fi
}

is_running_pid() {
  local pid="$1"
  if [ -z "${pid}" ]; then
    return 1
  fi
  if kill -0 "${pid}" 2>/dev/null; then
    return 0
  fi
  return 1
}

stop_existing() {
  local pid
  pid="$(read_pid)"

  if is_running_pid "${pid}"; then
    echo "stopping existing pid=${pid}"
    kill "${pid}" 2>/dev/null || true
    for _ in $(seq 1 50); do
      if ! is_running_pid "${pid}"; then
        break
      fi
      sleep 0.1
    done
  fi

  if [ -n "${pid}" ] && is_running_pid "${pid}"; then
    echo "force stopping pid=${pid}"
    kill -9 "${pid}" 2>/dev/null || true
  fi

  rm -f "${PID_PATH}" || true
}

start_new() {
  echo "starting backend..."
  nohup "${BIN_PATH}" >>"${LOG_PATH}" 2>&1 &
  local pid="$!"
  echo "${pid}" >"${PID_PATH}"
  disown || true

  sleep 0.2
  if ! is_running_pid "${pid}"; then
    echo "backend failed to start, see ${LOG_PATH}" >&2
    exit 1
  fi

  echo "started pid=${pid}"
}

stop_existing
start_new

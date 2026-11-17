#!/usr/bin/env bash
# Runs a built Wick on Linux without a desktop (run it under xvfb-run and dbus-run-session):
# it must start, keep the computer awake through logind, report status, and let go on stop
# and on kill -9. It also checks wick hold, wick release and wick run; WICK_SMOKE_HOLDS=0 skips them.
set -euo pipefail
bin=$1
inhibitors() { systemd-inhibit --list --no-pager 2>/dev/null | grep -c "Wick: " || true; }
status() { "$bin" status; }
# Polls a check every half second for at most $1 seconds.
wait_until() {
  local seconds=$1 what=$2 ticks=0
  shift 2
  until "$@"; do
    ((ticks++ < seconds * 2)) || { echo "$what (waited ${seconds}s)"; systemd-inhibit --list --no-pager; exit 1; }
    sleep 0.5
  done
}
held() { [[ "$(inhibitors)" -ge 1 ]]; }
released() { [[ "$(inhibitors)" -eq 0 ]]; }

check_holds() {
  "$bin" stop
  wait_until 15 "inhibitor left before the hold checks" released

  # Until the agents folder exists Wick reads it on the idle tick, once a minute.
  "$bin" hold smoke --pid "$$"
  wait_until 75 "no inhibitor while a hold works" held
  status | grep -q '"awake":true' || { echo "not awake while a hold works"; exit 1; }
  "$bin" release smoke
  wait_until 15 "inhibitor left after wick release" released
  status | grep -q '"awake":false' || { echo "still awake after wick release"; exit 1; }

  timeout 30 "$bin" run -- sleep 20 &
  local runner=$!
  wait_until 15 "no inhibitor while wick run's command runs" held
  wait "$runner" || { echo "wick run exited with $? for a command that succeeded"; exit 1; }
  wait_until 15 "inhibitor left after wick run's command ended" released

  local code=0
  timeout 30 "$bin" run -- sh -c 'exit 7' || code=$?
  [[ $code -eq 7 ]] || { echo "wick run exited with $code, expected the command's 7"; exit 1; }
  echo "holds: hold, release and run pass."
}

"$bin" --background >wick-smoke.log 2>&1 &
pid=$!
# Leave nothing running if a check fails.
trap 'kill -9 "$pid" 2>/dev/null || true; rm -f wick-status.json' EXIT
for _ in $(seq 1 30); do
  if status | grep -q '"running":true'; then break; fi
  sleep 1
done
status | grep -q '"running":true' || { echo "Wick did not start"; cat wick-smoke.log; exit 1; }
grep -q "opening as a window" wick-smoke.log || { echo "expected the no-tray window fallback"; cat wick-smoke.log; exit 1; }

"$bin" start 1
sleep 2
status | tee wick-status.json | grep -q '"awake":true' || { echo "not awake after start"; exit 1; }
[[ "$(inhibitors)" -ge 1 ]] || { echo "no logind inhibitor while awake"; systemd-inhibit --list --no-pager; exit 1; }
echo "awake: $(systemd-inhibit --list --no-pager | grep 'Wick: ')"

"$bin" stop
sleep 2
[[ "$(inhibitors)" -eq 0 ]] || { echo "inhibitor left after stop"; exit 1; }
status | grep -q '"awake":false' || { echo "still awake after stop"; exit 1; }

if [[ "${WICK_SMOKE_HOLDS:-1}" == 1 ]]; then check_holds; fi

"$bin" start
sleep 2
[[ "$(inhibitors)" -ge 1 ]] || { echo "no inhibitor for an open-ended session"; exit 1; }
# The pid Wick reports itself: an AppImage's launcher is a different process.
app_pid=$(status | sed -n 's/.*"pid":\([0-9]*\).*/\1/p')
kill -9 "${app_pid:-$pid}"
sleep 2
[[ "$(inhibitors)" -eq 0 ]] || { echo "inhibitor left after kill -9"; exit 1; }
status | grep -q '"running":false' || { echo "status still says running"; exit 1; }
echo "Linux smoke test passed."

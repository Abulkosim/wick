#!/usr/bin/env bash
# Runs a built Wick on Linux without a desktop (run it under xvfb-run and dbus-run-session):
# it must start, keep the computer awake through logind, report status, and let go on stop
# and on kill -9.
set -euo pipefail
bin=$1
inhibitors() { systemd-inhibit --list --no-pager 2>/dev/null | grep -c "Wick: " || true; }
status() { "$bin" status; }

"$bin" --background >wick-smoke.log 2>&1 &
pid=$!
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

"$bin" start
sleep 2
[[ "$(inhibitors)" -ge 1 ]] || { echo "no inhibitor for an open-ended session"; exit 1; }
kill -9 "$pid"
sleep 2
[[ "$(inhibitors)" -eq 0 ]] || { echo "inhibitor left after kill -9"; exit 1; }
status | grep -q '"running":false' || { echo "status still says running"; exit 1; }
echo "Linux smoke test passed."

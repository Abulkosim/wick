#!/usr/bin/env bash
# Runs a built Wick on macOS: it must start, hold a sleep assertion that powerd lists under its
# own pid while awake, report status, and let go on stop and on kill -9.
# Usage: macos-smoke.sh <Wick.app | Wick.app/Contents/MacOS/wick>
# Also checks wick hold, wick release and wick run; WICK_SMOKE_HOLDS=0 skips them.
set -euo pipefail
bin=${1:?Usage: macos-smoke.sh <Wick.app | path to its wick binary>}
[[ -d "$bin" ]] && bin="$bin/Contents/MacOS/wick"
work=$(mktemp -d)
log="$work/wick.log"
touch "$log"
pid=""
app_pid=""
runner=""
code=0

fail() {
  {
    echo "macOS smoke test failed: $*"
    echo "--- status"; cat "$work/status.json" 2>/dev/null || true
    echo "--- Wick log"; cat "$log"
    echo "--- pmset -g assertions"; pmset -g assertions || true
  } >&2
  exit 1
}

cleanup() {
  for process in "$runner" "$app_pid" "$pid"; do
    if [[ -n "$process" ]]; then kill -9 "$process" 2>/dev/null || true; fi
  done
  rm -rf "$work"
}
trap cleanup EXIT

[[ -x "$bin" ]] || fail "no Wick binary at $bin"

# Runs a command for at most $1 seconds. Output lands in $work/out, the exit code in $code.
limited() {
  local seconds=$1 ticks=0
  shift
  "$@" >"$work/out" 2>"$work/err" &
  local child=$!
  while kill -0 "$child" 2>/dev/null; do
    if ((ticks++ >= seconds * 10)); then
      kill -9 "$child" 2>/dev/null || true
      fail "$* did not finish within ${seconds}s"
    fi
    sleep 0.1
  done
  code=0
  wait "$child" || code=$?
}

wick() {
  limited 30 "$bin" "$@"
  ((code == 0)) || fail "wick $* exited with $code: $(cat "$work/err")"
}

status() {
  wick status
  cp "$work/out" "$work/status.json"
}

expect() {
  grep -q "\"$1\":$2" "$work/status.json" || fail "$3: expected \"$1\":$2"
}

# Polls a check every half second for at most $1 seconds.
wait_until() {
  local seconds=$1 what=$2 ticks=0
  shift 2
  until "$@"; do
    ((ticks++ < seconds * 2)) || fail "$what (waited ${seconds}s)"
    sleep 0.5
  done
}

# Wick's assertions in powerd's list, any owner.
wick_assertions() { pmset -g assertions | grep -F 'named: "Wick: ' || true; }
# Wick's assertions owned by the running app, in powerd's list.
owned() { pmset -g assertions | grep -E "^[[:space:]]*pid ${app_pid}\(" | grep -F 'named: "Wick: ' || true; }
held() { [[ -n "$(owned)" ]]; }
released() { [[ -z "$(wick_assertions)" ]]; }
running() { status && grep -q '"running":true' "$work/status.json"; }

# keepDisplayOn picks the assertion type: display sleep, or only system sleep.
expect_assertion() {
  local type=PreventUserIdleSystemSleep
  grep -q '"keepDisplayOn":true' "$work/status.json" && type=PreventUserIdleDisplaySleep
  wait_until "${2:-15}" "$1: no Wick assertion owned by pid $app_pid" held
  owned | grep -q "$type" || fail "$1: expected a $type assertion, got: $(owned)"
  echo "awake: $(owned | sed 's/^[[:space:]]*//')"
}

check_holds() {
  wick stop
  wait_until 15 "assertion left before the hold checks" released

  # Until the agents folder exists Wick reads it on the idle tick, once a minute.
  wick hold smoke --pid "$$"
  expect_assertion "wick hold" 75
  status
  expect awake true "not awake while a hold works"
  wick release smoke
  wait_until 15 "assertion left after wick release" released
  status
  expect awake false "still awake after wick release"

  "$bin" run -- /bin/sleep 20 >"$work/run.log" 2>&1 &
  runner=$!
  expect_assertion "wick run"
  local ticks=0
  while kill -0 "$runner" 2>/dev/null; do
    ((ticks++ < 300)) || fail "wick run -- /bin/sleep 20 did not finish within 30s"
    sleep 0.1
  done
  code=0
  wait "$runner" || code=$?
  runner=""
  ((code == 0)) || fail "wick run exited with $code for a command that succeeded: $(cat "$work/run.log")"
  wait_until 15 "assertion left after wick run's command ended" released

  limited 30 "$bin" run -- /bin/sh -c 'exit 7'
  ((code == 7)) || fail "wick run exited with $code, expected the command's 7"
  echo "holds: hold, release and run pass."
}

"$bin" --background >"$log" 2>&1 &
pid=$!
disown "$pid"
wait_until 30 "Wick did not start" running
app_pid=$(sed -n 's/.*"pid":\([0-9]*\).*/\1/p' "$work/status.json")
[[ "$app_pid" == "$pid" ]] || fail "status names pid ${app_pid:-none}, but Wick runs as $pid"
expect awake false "awake before any start"
released || fail "a Wick assertion exists before any start: $(wick_assertions)"

wick start 1
status
expect awake true "not awake after start 1"
expect mode '"for"' "start 1 is not a timed session"
expect_assertion "start 1"

wick stop
wait_until 15 "assertion left after stop" released
status
expect awake false "still awake after stop"
expect mode '"off"' "session still on after stop"

if [[ "${WICK_SMOKE_HOLDS:-1}" == 1 ]]; then check_holds; fi

wick start
status
expect awake true "not awake after start"
expect mode '"indefinite"' "start is not open-ended"
expect_assertion "start"

kill -9 "$app_pid"
wait_until 15 "assertion left after kill -9" released
status
expect running false "status still says running after kill -9"
expect awake false "status still says awake after kill -9"
echo "macOS smoke test passed."

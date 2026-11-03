#!/usr/bin/env bash
# Run every client's self-test on a real Ubuntu 24.04 machine (x86_64 or
# arm64) over SSH, to exercise the hardware paths (RDRAND / RNDR) that a
# development laptop may not have. Build first with scripts/build-clients.sh.
#
#   scripts/test-clients-remote.sh ubuntu@HOST [API_URL ...]
#
# API_URLs default to https://rdrand.com/API/. Installs nodejs and a JRE on
# the target with apt. Set SSH_KEY to use a specific private key.
#
# Verified for 2.0.0 on AWS: t3.small (Intel Xeon, rdrand), c7g.medium
# (Graviton3, rndr) and t4g.small (Graviton2, no RNG: API fallback).
set -euo pipefail
cd "$(dirname "$0")/.."
TARGET=${1:?usage: test-clients-remote.sh user@host [api-url ...]}
shift
URLS=("${@:-https://rdrand.com/API/}")
SSH_OPTS=(-o BatchMode=yes -o ConnectTimeout=10 -o StrictHostKeyChecking=accept-new)
[[ -n "${SSH_KEY:-}" ]] && SSH_OPTS+=(-i "$SSH_KEY")

ssh "${SSH_OPTS[@]}" "$TARGET" 'rm -rf ~/rdrand-client-tests && mkdir -p ~/rdrand-client-tests'
scp -q "${SSH_OPTS[@]}" -r dist/client-tests/* dist/jebtek-rdrand-2.0.0.tgz "$TARGET:rdrand-client-tests/"

# shellcheck disable=SC2029 # URLS is meant to expand here, on the client.
ssh "${SSH_OPTS[@]}" "$TARGET" "URLS='${URLS[*]}' bash -s" <<'EOF'
set -u
cd ~/rdrand-client-tests
if ! command -v node >/dev/null || ! command -v java >/dev/null; then
  sudo apt-get update -qq >/dev/null
  sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq nodejs openjdk-17-jre-headless >/dev/null 2>&1
fi
arch=$(uname -m); rid=$([ "$arch" = x86_64 ] && echo linux-x64 || echo linux-arm64)
chmod +x rust-$arch/* dotnet-selftest-$rid/SelfTest
# The Node test runs against the exact contents of the downloadable tarball.
mkdir -p node-pkg && tar xzf jebtek-rdrand-2.0.0.tgz -C node-pkg && cp -R node-test node-pkg/package/test

echo "CPU: $(lscpu | sed -n 's/^Model name: *//p' | head -1) ($arch)"
echo "cpuinfo: rdrand=$(grep -c -w rdrand /proc/cpuinfo) rng=$(grep -m1 -E '^(Features|flags)' /proc/cpuinfo | grep -c -w rng)"
fails=0
run() { # label command...
  local out rc
  out=$("${@:2}" 2>&1); rc=$?
  local summary
  summary=$(echo "$out" | grep -E 'test result|^# (pass|fail)|all checks passed|FAILED|FAIL\]|hardware generator:' | tr '\n' ' ')
  printf '  %-34s %s %s\n' "$1" "$([ $rc -eq 0 ] && echo PASS || echo FAIL)" "$summary"
  [ $rc -eq 0 ] || { fails=$((fails + 1)); echo "$out" | tail -20 | sed 's/^/      /'; }
}
echo "== hardware"
./rust-$arch/demo 2>&1 | head -1 | sed 's/^/  rust demo: /'
run "rust unit (hardware)" ./rust-$arch/rdrand_client-unit -q
for url in $URLS; do
  echo "== API fallback: $url"
  export RDRAND_TEST_URL=$url
  run "rust service" ./rust-$arch/service -q
  run "node" sh -c 'cd node-pkg/package && node --test'
  run "kotlin (JVM)" java -jar rdrand-kotlin-selftest-2.0.0.jar
  run ".NET" ./dotnet-selftest-$rid/SelfTest
done
echo "== $( [ $fails -eq 0 ] && echo 'ALL PASSED' || echo "$fails FAILED")"
exit $fails
EOF

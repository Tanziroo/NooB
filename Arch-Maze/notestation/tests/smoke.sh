#!/usr/bin/env bash
# Smoke tests for the Python tools (dial, sealgate). No dependencies.
# Runs the real end-to-end flows and asserts exit codes / behavior. CI runs this.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
DIAL="python3 $root/dial/dial.py"
GATE="python3 $root/sealgate/sealgate.py"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
cd "$tmp"

echo "── dial ──"
$DIAL init --mode forensic >/dev/null
$DIAL new --task "smoke test" >/dev/null
sid="$(ls .dial/sessions)"
printf 'canonical record\n' > ".dial/sessions/$sid/record.txt"
$DIAL seal "$sid" >/dev/null
$DIAL status | grep -q sealed
echo "  dial init/new/seal/resume ...... OK"

echo "── sealgate ──"
printf 'contract artifact bytes' > deal.txt
cat > contract.json <<'JSON'
{ "level": "L2", "require": { "content_hash": true, "signer_perm": "L2" } }
JSON

# 1) fail closed: an unconfigured layer must REFUSE (nonzero exit)
if $GATE seal contract.json deal.txt >/dev/null 2>&1; then
  echo "  FAIL: sealed with an unverified layer"; exit 1
fi
echo "  fail-closed on unverified layer  OK"

# 2) with a verifier that approves, it seals (zero exit)
cat > dev.sh <<'SH'
#!/usr/bin/env bash
c=$(cat | grep -o '"clause": *"[^"]*"' | cut -d'"' -f4)
echo "{\"ok\": true, \"detail\": \"$c ok\"}"
SH
chmod +x dev.sh
printf '{ "signer_perm": "%s/dev.sh" }' "$tmp" > verifiers.json
$GATE seal contract.json deal.txt --verifiers verifiers.json >/dev/null
echo "  seal when every layer passes ... OK"

# 3) verify unchanged passes; tamper is detected
$GATE verify deal.txt >/dev/null
printf 'X' >> deal.txt
if $GATE verify deal.txt >/dev/null 2>&1; then
  echo "  FAIL: tamper not detected"; exit 1
fi
echo "  tamper detection ............... OK"

echo ""
echo "ALL SMOKE TESTS PASSED"

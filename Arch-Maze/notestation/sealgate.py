#!/usr/bin/env python3
"""
sealgate — the Notestation assembly spine.

A contract declares the attestation LAYERS it requires. sealgate refuses to seal
an artifact until EVERY required layer is satisfied by a REAL mechanism. It never
fakes a layer it cannot verify: an unconfigured clause FAILS CLOSED (Label <=
Mechanism, enforced in code).

Your proven pieces plug in as external VERIFIER COMMANDS. sealgate calls a verifier
with the clause + artifact context as JSON on stdin and reads {"ok", "detail"} on
stdout — the same honest-executor split scribe/dial use. Built-in verifiers cover
what code can do directly (content_hash); everything else is yours to supply.

  sealgate check <contract.json> <artifact> [--verifiers map.json]
  sealgate seal  <contract.json> <artifact> [--verifiers map.json]
  sealgate init                     scaffold a sample contract + verifier map

Stdlib only. Real SHA-256 via hashlib.

Contract format (contract.json):
  { "level": "L2",
    "require": {
      "content_hash": true,            # built-in: seal binds the artifact's sha256
      "signer_perm": "L2",             # external: your lattice verifies permission
      "host_in": ["desk-A"],           # external: your host-anchor check
      "cosign": 1                      # external: N co-signers
    } }

Verifier map (verifiers.json):  clause -> command (receives ctx JSON on stdin)
  { "signer_perm": "/path/to/device verify-signer",
    "host_in":     "/path/to/device verify-host",
    "cosign":      "/path/to/device cosign" }
"""

import argparse
import datetime as _dt
import hashlib
import json
import os
import subprocess
import sys

GATE_VERSION = "0.1"
AUDIT = "sealgate-audit.log"


def _now():
    return _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def _atomic_write(path, text):
    tmp = path + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        f.write(text)
        f.flush()
        os.fsync(f.fileno())
    os.replace(tmp, path)


def _append(path, line):
    with open(path, "a", encoding="utf-8") as f:
        f.write(line + "\n")
        f.flush()
        os.fsync(f.fileno())


def _sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


# ---------------------------------------------------------------------------
# built-in verifiers: things code can prove directly. Each returns (ok, detail).
# ---------------------------------------------------------------------------
def _v_content_hash(value, ctx):
    got = ctx["artifact_sha256"]
    if value is True:
        return True, f"sha256={got} (bound)"
    if isinstance(value, str):
        ok = got.lower() == value.lower()
        return ok, f"sha256={got} {'==' if ok else '!='} expected"
    return False, "content_hash: value must be true or an expected sha256 hex"


BUILTINS = {
    "content_hash": _v_content_hash,
}


def _run_external(cmd, clause, value, ctx):
    """Call an operator-supplied verifier. It reads ctx JSON on stdin, prints
    {"ok": bool, "detail": str} on stdout. Any failure = fail closed."""
    payload = json.dumps({"clause": clause, "value": value, **ctx})
    try:
        parts = cmd.split()
        p = subprocess.run(parts, input=payload, capture_output=True, text=True, timeout=60)
    except Exception as e:  # noqa: BLE001
        return False, f"verifier error: {e}"
    if p.returncode != 0 and not p.stdout.strip():
        return False, f"verifier exit {p.returncode}: {p.stderr.strip()[:120]}"
    try:
        out = json.loads(p.stdout.strip() or "{}")
    except json.JSONDecodeError:
        return False, f"verifier returned non-JSON: {p.stdout.strip()[:80]}"
    return bool(out.get("ok")), str(out.get("detail", ""))


def _evaluate(contract, artifact, verifiers):
    if not os.path.exists(artifact):
        sys.exit(f"sealgate: artifact not found: {artifact}")
    ctx = {"artifact": artifact, "artifact_sha256": _sha256(artifact)}
    require = contract.get("require", {})
    results = []
    for clause, value in require.items():
        if clause in BUILTINS:
            ok, detail = BUILTINS[clause](value, ctx)
            src = "built-in"
        elif clause in verifiers:
            ok, detail = _run_external(verifiers[clause], clause, value, ctx)
            src = "external"
        else:
            # FAIL CLOSED: a layer we cannot verify is never assumed satisfied.
            ok, detail = False, "NO VERIFIER CONFIGURED — fail closed"
            src = "none"
        results.append({"clause": clause, "value": value, "ok": ok, "src": src, "detail": detail})
    passed = all(r["ok"] for r in results) and len(results) > 0
    return ctx, results, passed


def _print_report(contract, ctx, results, passed):
    print(f"contract level: {contract.get('level', '?')}   artifact: {ctx['artifact']}")
    print(f"artifact sha256: {ctx['artifact_sha256']}")
    print("clauses:")
    for r in results:
        mark = "PASS" if r["ok"] else "FAIL"
        print(f"  [{mark}] {r['clause']} ({r['src']}) = {json.dumps(r['value'])}  — {r['detail']}")
    print(f"VERDICT: {'SEAL PERMITTED' if passed else 'REFUSED'}")


def _load(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def cmd_check(args):
    contract = _load(args.contract)
    verifiers = _load(args.verifiers) if args.verifiers and os.path.exists(args.verifiers) else {}
    ctx, results, passed = _evaluate(contract, args.artifact, verifiers)
    _print_report(contract, ctx, results, passed)
    sys.exit(0 if passed else 1)


def cmd_seal(args):
    contract = _load(args.contract)
    verifiers = _load(args.verifiers) if args.verifiers and os.path.exists(args.verifiers) else {}
    ctx, results, passed = _evaluate(contract, args.artifact, verifiers)
    _print_report(contract, ctx, results, passed)
    if not passed:
        unmet = [r["clause"] for r in results if not r["ok"]]
        _append(AUDIT, f"{_now()} REFUSED artifact={args.artifact} sha256={ctx['artifact_sha256']} unmet={','.join(unmet)}")
        print(f"sealgate: REFUSED — unmet layers: {', '.join(unmet)}. Nothing sealed.")
        sys.exit(1)
    record = {
        "gate_version": GATE_VERSION,
        "sealed_at": _now(),
        "level": contract.get("level"),
        "artifact": args.artifact,
        "artifact_sha256": ctx["artifact_sha256"],
        "clauses": [{"clause": r["clause"], "value": r["value"], "detail": r["detail"]} for r in results],
    }
    out = args.artifact + ".attestation.json"
    _atomic_write(out, json.dumps(record, indent=2) + "\n")
    _append(AUDIT, f"{_now()} SEALED artifact={args.artifact} sha256={ctx['artifact_sha256']} level={contract.get('level')} layers={len(results)}")
    print(f"sealgate: SEALED — attestation written to {out}")


def cmd_verify(args):
    """Re-check a sealed artifact against ITS attestation: has it changed since?"""
    att_path = args.artifact + ".attestation.json"
    if not os.path.exists(att_path):
        sys.exit(f"sealgate: no attestation found ({att_path}) — artifact was never sealed")
    att = _load(att_path)
    if not os.path.exists(args.artifact):
        print(f"MISSING: {args.artifact} (sealed sha256 {att['artifact_sha256']})")
        sys.exit(1)
    got = _sha256(args.artifact)
    want = att["artifact_sha256"]
    ok = got == want
    print(f"artifact: {args.artifact}")
    print(f"  sealed sha256: {want}")
    print(f"  current sha256: {got}")
    print(f"  level {att.get('level')} · {len(att.get('clauses', []))} layers · sealed {att.get('sealed_at')}")
    print("VERIFIED: unchanged since seal." if ok else "TAMPERED: artifact differs from its sealed attestation.")
    sys.exit(0 if ok else 1)


def cmd_init(args):
    contract = {
        "level": "L2",
        "require": {
            "content_hash": True,
            "signer_perm": "L2",
            "host_in": ["desk-A"],
        },
    }
    verifiers = {
        "signer_perm": "/path/to/your-device verify-signer",
        "host_in": "/path/to/your-device verify-host",
    }
    _atomic_write("contract.json", json.dumps(contract, indent=2) + "\n")
    _atomic_write("verifiers.json", json.dumps(verifiers, indent=2) + "\n")
    print("sealgate: wrote contract.json + verifiers.json (edit verifier commands to point at your device).")


def main(argv=None):
    p = argparse.ArgumentParser(prog="sealgate", description="Refuse to seal until every required attestation layer is really satisfied.")
    sub = p.add_subparsers(dest="cmd", required=True)

    pc = sub.add_parser("check", help="evaluate a contract against an artifact (no seal)")
    pc.add_argument("contract")
    pc.add_argument("artifact")
    pc.add_argument("--verifiers")
    pc.set_defaults(func=cmd_check)

    ps = sub.add_parser("seal", help="seal only if every layer passes")
    ps.add_argument("contract")
    ps.add_argument("artifact")
    ps.add_argument("--verifiers")
    ps.set_defaults(func=cmd_seal)

    pv = sub.add_parser("verify", help="re-check a sealed artifact against its attestation")
    pv.add_argument("artifact")
    pv.set_defaults(func=cmd_verify)

    sub.add_parser("init", help="scaffold a sample contract + verifier map").set_defaults(func=cmd_init)

    args = p.parse_args(argv)
    args.func(args)


if __name__ == "__main__":
    main()

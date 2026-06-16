#!/usr/bin/env bash
# Smoke-test the deployed soter-server.
#
# ─── Canonical flow: Cloud Shell ──────────────────────────────────────────────
# The soter-server Cloud Run service is `ingress=internal`, so callers must
# reach it from inside the VPC. Cloud Shell (https://shell.cloud.google.com)
# runs in Google's network, ships grpcurl, and uses your authenticated
# identity by default.
#
#   1. Open Cloud Shell, project = reflective-labs
#   2. git clone --depth=1 -b next https://github.com/Reflective-Lab/soter-smt.git
#   3. cd soter-smt
#   4. ops/smoke.sh https://soter-server-640630843925.europe-west1.run.app
#
# tonic-reflection is registered server-side, so no -proto flag is needed.
#
# ─── Alternative: ingress flip ────────────────────────────────────────────────
# If Cloud Shell isn't available, temporarily flip `--ingress=all` (auth
# still required via ID token) for a 3-minute window:
#
#   gcloud run services update soter-server --project=reflective-labs \
#       --region=europe-west1 --ingress=all
#   TOKEN=$(gcloud auth print-identity-token)
#   grpcurl -H "authorization: Bearer ${TOKEN}" \
#       soter-server-640630843925.europe-west1.run.app:443 list
#   # ...smoke...
#   gcloud run services update soter-server --project=reflective-labs \
#       --region=europe-west1 --ingress=internal
#
# ─── grpcurl install ──────────────────────────────────────────────────────────
# Cloud Shell: pre-installed.
# macOS:      brew install grpcurl
# Linux:      see https://github.com/fullstorydev/grpcurl/releases
set -euo pipefail

URL="${1:?usage: smoke.sh <soter-server-url>}"
HOST_PORT="${URL#https://}"
HOST_PORT="${HOST_PORT#http://}"
HOST_PORT="${HOST_PORT%/}:443"
SCHEME_FLAGS=""
[[ "$URL" == http://* ]] && SCHEME_FLAGS="-plaintext"

echo "── 1/5: grpc.health.v1.Health/Check should return SERVING ──"
RESP=$(grpcurl $SCHEME_FLAGS -d '{"service": ""}' "$HOST_PORT" grpc.health.v1.Health/Check)
echo "$RESP"
echo "$RESP" | grep -q '"status": "SERVING"' \
    || { echo "FAIL: expected SERVING"; exit 1; }
echo "ok"
echo

echo "── 2/5: list should show three services ──"
LIST=$(grpcurl $SCHEME_FLAGS "$HOST_PORT" list)
echo "$LIST"
for svc in "grpc.health.v1.Health" "grpc.reflection.v1.ServerReflection" "soter.v1.SoterSolver"; do
    echo "$LIST" | grep -q "^${svc}$" \
        || { echo "FAIL: missing service $svc"; exit 1; }
done
echo "ok"
echo

echo "── 3/5: Check WITHOUT x-converge-app → INVALID_ARGUMENT ──"
if grpcurl $SCHEME_FLAGS \
       -d '{"query":{"query_id":"smoke","smtlib":"(check-sat)","timeout_ms":5000}}' \
       "$HOST_PORT" soter.v1.SoterSolver/Check 2>&1 \
       | grep -q "InvalidArgument"; then
    echo "ok"
else
    echo "FAIL: expected InvalidArgument for missing tenant header"
    exit 1
fi
echo

echo "── 4/5: Check WITH unknown tenant → PERMISSION_DENIED ──"
if grpcurl $SCHEME_FLAGS \
       -H 'x-converge-app: nope-not-real' \
       -d '{"query":{"query_id":"smoke","smtlib":"(check-sat)","timeout_ms":5000}}' \
       "$HOST_PORT" soter.v1.SoterSolver/Check 2>&1 \
       | grep -q "PermissionDenied"; then
    echo "ok"
else
    echo "FAIL: expected PermissionDenied for unknown tenant"
    exit 1
fi
echo

echo "── 5/5: Check WITH quorum-sense + trivial (check-sat) ──"
RESP=$(grpcurl $SCHEME_FLAGS \
    -H 'x-converge-app: quorum-sense' \
    -d '{"query":{"query_id":"smoke","smtlib":"(check-sat)","timeout_ms":5000}}' \
    "$HOST_PORT" soter.v1.SoterSolver/Check)
echo "$RESP"
echo "$RESP" | grep -q '"status"' \
    || { echo "FAIL: expected structured SmtReport"; exit 1; }
echo "ok"
echo

echo "── ALL 5 SMOKE CHECKS PASSED ──"

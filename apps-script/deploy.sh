#!/usr/bin/env bash
# Push Apps Script code and bump the live deployment, fully automated.
#
# Why this exists: Apps Script's `deployments.update` API and `clasp deploy -i`
# both reset Web App entry points (deployment becomes a Library, URL 404s).
# We work around that by creating a NEW deployment each time and updating the
# API URL referenced in the repo. Old deployments are left intact for rollback.
#
# Usage: ./deploy.sh ["description"]

set -euo pipefail

cd "$(dirname "$0")"
REPO_ROOT="$(cd .. && pwd)"
INDEX="$REPO_ROOT/index.html"
CLAUDE="$REPO_ROOT/CLAUDE.md"
TOKEN_FILE="$HOME/.clasprc.json"
DESC="${1:-deploy $(date +%Y-%m-%d)}"

if [[ ! -f "$TOKEN_FILE" ]]; then
  echo "error: $TOKEN_FILE not found — run 'npx clasp login' first" >&2
  exit 1
fi

SCRIPT_ID=$(python3 -c "import json; print(json.load(open('.clasp.json'))['scriptId'])")

read -r CLIENT_ID CLIENT_SECRET REFRESH_TOKEN < <(python3 -c "
import json
d = json.load(open('$TOKEN_FILE'))['tokens']['default']
print(d['client_id'], d['client_secret'], d['refresh_token'])
")

ACCESS=$(curl -sS -X POST https://oauth2.googleapis.com/token \
  -d "client_id=$CLIENT_ID" \
  -d "client_secret=$CLIENT_SECRET" \
  -d "refresh_token=$REFRESH_TOKEN" \
  -d "grant_type=refresh_token" \
  | python3 -c "import json,sys; print(json.load(sys.stdin)['access_token'])")

echo "→ Pushing code to HEAD..."
npx clasp push --force >/dev/null

# If a version was passed in via env (e.g., retry after partial failure), reuse.
VERSION="${REUSE_VERSION:-}"
if [[ -z "$VERSION" ]]; then
  echo "→ Creating new version..."
  VERSION_BODY=$(python3 -c 'import json,sys; print(json.dumps({"description": sys.argv[1]}))' "$DESC")
  VERSION=$(curl -sS -X POST -H "Authorization: Bearer $ACCESS" -H "Content-Type: application/json" \
    -d "$VERSION_BODY" \
    "https://script.googleapis.com/v1/projects/$SCRIPT_ID/versions" \
    | python3 -c "import json,sys; print(json.load(sys.stdin)['versionNumber'])")
fi
echo "  v$VERSION"

echo "→ Creating new deployment..."
DEP_BODY=$(python3 -c '
import json, sys
print(json.dumps({
  "scriptId": sys.argv[1],
  "versionNumber": int(sys.argv[2]),
  "manifestFileName": "appsscript",
  "description": sys.argv[3],
}))
' "$SCRIPT_ID" "$VERSION" "$DESC")

NEW=$(curl -sS -X POST -H "Authorization: Bearer $ACCESS" -H "Content-Type: application/json" \
  -d "$DEP_BODY" \
  "https://script.googleapis.com/v1/projects/$SCRIPT_ID/deployments")

DEP_ID=$(echo "$NEW" | python3 -c "import json,sys; print(json.load(sys.stdin)['deploymentId'])")
NEW_URL="https://script.google.com/macros/s/$DEP_ID/exec"
echo "  $NEW_URL"

echo "→ Updating $INDEX and $CLAUDE..."
python3 - "$INDEX" "$CLAUDE" "$NEW_URL" <<'PY'
import re, sys
url = sys.argv[3]
pat = re.compile(r'https://script\.google\.com/macros/s/[A-Za-z0-9_-]+/exec')
for path in sys.argv[1:3]:
    with open(path) as f:
        content = f.read()
    new = pat.sub(url, content)
    if new != content:
        with open(path, 'w') as f:
            f.write(new)
        print(f"  updated {path}")
    else:
        print(f"  no change in {path}")
PY

echo ""
echo "✓ Live at $NEW_URL"

#!/usr/bin/env bash
# trikala template verifier — checks the three load-bearing template rules
# from AGENT.md (F29 line cap, F30 forbidden wrappers, F21 dep pinning).
#
# Usage:
#   bash check-template.sh [template-dir]    # defaults to current directory
#
# Exit code:
#   0 = all checks passed
#   1 = at least one check failed
#   2 = invalid invocation (e.g. target dir missing)
#
# Run this before submitting code that touches a template's src/ or
# Cargo.toml. If you cannot run shell, perform the same three checks
# manually — the rule is what matters, not the tool.

set -u

DIR="${1:-.}"
if [ ! -d "$DIR" ]; then
  echo "ERR: '$DIR' is not a directory" >&2
  exit 2
fi
cd "$DIR" || exit 2

FAIL=0

# ---------------------------------------------------------------------------
# F29 — template line-count cap: ≤ 300 .rs lines under src/
# ---------------------------------------------------------------------------
if [ -d src ]; then
  LINES=$(find src -name '*.rs' -type f -exec cat {} + 2>/dev/null | wc -l)
  LINES=$(echo "$LINES" | tr -d ' ')
  if [ "${LINES:-0}" -gt 300 ]; then
    echo "FAIL F29: src/ has $LINES .rs lines (cap is 300)"
    FAIL=1
  else
    echo "OK   F29: $LINES / 300 .rs lines"
  fi
else
  echo "SKIP F29: no src/ directory in '$DIR'"
fi

# ---------------------------------------------------------------------------
# F30 — no trikala-* wrapper imports (only trikala-core is allowed)
# ---------------------------------------------------------------------------
FORBIDDEN_RE='trikala-(render|audio|text|ui|assets|save|net)'
F30_HITS=""
if [ -f Cargo.toml ]; then
  HITS_TOML=$(grep -REn "$FORBIDDEN_RE" Cargo.toml 2>/dev/null || true)
  F30_HITS="$HITS_TOML"
fi
if [ -d src ]; then
  HITS_SRC=$(grep -REn "$FORBIDDEN_RE" src 2>/dev/null || true)
  if [ -n "$HITS_SRC" ]; then
    F30_HITS="${F30_HITS}
${HITS_SRC}"
  fi
fi
if [ -n "$(echo "$F30_HITS" | tr -d '[:space:]')" ]; then
  echo "FAIL F30: forbidden trikala-* wrapper imported (only trikala-core is allowed):"
  echo "$F30_HITS" | sed 's/^/       /'
  FAIL=1
else
  echo "OK   F30: no forbidden trikala-* imports"
fi

# ---------------------------------------------------------------------------
# F21 — every dependency version must be pinned "=x.y.z"
# ---------------------------------------------------------------------------
if [ -f Cargo.toml ]; then
  UNPINNED=$(awk '
    /^\[/ {
      in_deps = ($0 ~ /^\[(dev-|build-)?dependencies(\..+)?\]$/) || \
                ($0 ~ /^\[workspace\.dependencies(\..+)?\]$/)
      next
    }
    in_deps {
      line = $0
      sub(/#.*/, "", line)
      if (match(line, /=[ \t]*"[^"]+"/)) {
        v = substr(line, RSTART, RLENGTH)
        sub(/^=[ \t]*"/, "", v)
        sub(/"$/, "", v)
        # Flag bare-number, ^, or ~ prefixes (cargo treats bare as ^).
        # Skip git URLs, paths, booleans, etc. — those donot start with [0-9^~].
        if (v ~ /^[\^~0-9]/ && v !~ /^=/) {
          print FILENAME ":" NR ": " $0
        }
      }
    }
  ' Cargo.toml)
  if [ -n "$UNPINNED" ]; then
    echo "FAIL F21: unpinned deps (use \"=x.y.z\"):"
    echo "$UNPINNED" | sed 's/^/       /'
    FAIL=1
  else
    echo "OK   F21: all deps pinned with =x.y.z"
  fi
fi

echo ""
if [ "$FAIL" -eq 0 ]; then
  echo "All template checks passed."
else
  echo "Template checks FAILED. Fix the issues above before submitting."
fi
exit "$FAIL"

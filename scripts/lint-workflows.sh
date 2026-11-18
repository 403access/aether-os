#!/usr/bin/env bash
#
# Static lint for .github/workflows: expression syntax, action names and
# inputs, `needs` references, runner labels and the shell inside `run:` blocks.
# This catches the errors that otherwise cost a push and a red run.
#
#   ./scripts/lint-workflows.sh
#   ./scripts/lint-workflows.sh -format '{{file}}:{{line}}: {{message}}'
#
# Requires actionlint (https://github.com/rhysd/actionlint). ShellCheck is
# optional but recommended: without it actionlint still checks the workflow,
# just not the body of each run step.
#
# Exit status: 0 clean, 1 findings, 2 usage/environment error.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}" || exit 2

WORKFLOWS=()
for path in .github/workflows/*.yml .github/workflows/*.yaml; do
  [ -e "${path}" ] && WORKFLOWS+=("${path}")
done

if [ "${#WORKFLOWS[@]}" -eq 0 ]; then
  echo "no workflow files under .github/workflows"
  exit 0
fi

if ! command -v actionlint >/dev/null 2>&1; then
  cat >&2 <<'EOF'
error: actionlint not installed

  brew install actionlint shellcheck
  # or: go install github.com/rhysd/actionlint/cmd/actionlint@latest
  # or: nix run nixpkgs#actionlint
EOF
  exit 2
fi

if ! command -v shellcheck >/dev/null 2>&1; then
  echo "note: shellcheck not found; run steps are not linted (brew install shellcheck)" >&2
fi

echo "Aether workflow lint · ${#WORKFLOWS[@]} file(s)"
echo

actionlint "$@" "${WORKFLOWS[@]}"
STATUS=$?

echo
if [ "${STATUS}" -eq 0 ]; then
  echo "RESULT: clean"
  exit 0
fi
echo "RESULT: findings"
exit 1
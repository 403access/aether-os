#!/usr/bin/env bash
#
# First-time publish to GitHub, then trigger the Pages workflow.
# Requires: gh auth login, a clean working tree.
#
#   ./scripts/publish.sh
#   REPO_NAME=aether-os VISIBILITY=public ./scripts/publish.sh
#
set -euo pipefail

REPO_NAME="${REPO_NAME:-aether-os}"
VISIBILITY="${VISIBILITY:-private}"
BRANCH="${BRANCH:-main}"

if ! command -v gh >/dev/null 2>&1; then
  echo "error: GitHub CLI (gh) is not installed" >&2
  exit 1
fi

if ! gh auth status >/dev/null 2>&1; then
  echo "error: not authenticated. Run: gh auth login" >&2
  exit 1
fi

if [ -n "$(git status --porcelain)" ]; then
  echo "error: working tree is dirty; commit or stash first" >&2
  git status --short >&2
  exit 1
fi

if ! git rev-parse --verify "$BRANCH" >/dev/null 2>&1; then
  echo "error: branch '$BRANCH' does not exist" >&2
  exit 1
fi

if git remote get-url origin >/dev/null 2>&1; then
  echo "==> origin already configured: $(git remote get-url origin)"
else
  echo "==> creating repository $REPO_NAME ($VISIBILITY)"
  gh repo create "$REPO_NAME" "--$VISIBILITY" --source=. --remote=origin --push
fi

echo "==> pushing $BRANCH"
git push -u origin "$BRANCH"

OWNER="$(gh repo view --json owner --jq .owner.login)"
echo "==> enabling GitHub Pages (GitHub Actions source)"
gh api -X POST "repos/$OWNER/$REPO_NAME/pages" -f build_type=workflow >/dev/null 2>&1 || \
  echo "    Pages already enabled or requires manual setup"

echo "==> triggering the Pages workflow"
gh workflow run pages.yml >/dev/null 2>&1 || \
  echo "    Could not trigger automatically; push a commit or run the workflow manually"

echo
echo "Repository: https://github.com/$OWNER/$REPO_NAME"
echo "Site:       https://$OWNER.github.io/$REPO_NAME/"
echo "Docs:       https://$OWNER.github.io/$REPO_NAME/docs/"
echo "Gallery:    https://$OWNER.github.io/$REPO_NAME/design-system/"

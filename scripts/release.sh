#!/usr/bin/env bash
set -euo pipefail

# ── LLM Gateway Release Script ──
# Usage: ./scripts/release.sh <version>
# Example: ./scripts/release.sh 0.9.7

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

die() { echo -e "${RED}ERROR:${NC} $*" >&2; exit 1; }
info() { echo -e "${GREEN}==>${NC} $*"; }
warn() { echo -e "${YELLOW}WARN:${NC} $*"; }

# ── Validate input ──
VERSION="${1:-}"
[[ -z "$VERSION" ]] && die "Usage: $0 <version>  (e.g. $0 0.9.7)"
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "Version must be semver (e.g. 0.9.7), got: $VERSION"

TAG="v${VERSION}"
RELEASE_BRANCH="release/${VERSION}"

# ── Pre-flight checks ──
info "Pre-flight checks"

git rev-parse --is-inside-work-tree >/dev/null || die "Not inside a git repository"

CURRENT=$(git branch --show-current)
[[ "$CURRENT" != "develop" ]] && die "Must be on 'develop' branch, currently on '$CURRENT'"

git fetch origin
LOCAL=$(git rev-parse develop)
REMOTE=$(git rev-parse origin/develop)
[[ "$LOCAL" != "$REMOTE" ]] && die "develop is out of sync with origin. git pull or git push first."

# ── Check for uncommitted changes ──
UNSTAGED=$(git diff --name-only)
STAGED=$(git diff --cached --name-only)
UNTRACKED=$(git ls-files --others --exclude-standard)

if [[ -n "$UNSTAGED" || -n "$STAGED" || -n "$UNTRACKED" ]]; then
    echo ""
    warn "You have uncommitted changes:"
    [[ -n "$STAGED" ]] && echo "  Staged:" && echo "    $STAGED" | tr ' ' '\n' | sed 's/^/    /'
    [[ -n "$UNSTAGED" ]] && echo "  Unstaged:" && echo "$UNSTAGED" | sed 's/^/    /'
    [[ -n "$UNTRACKED" ]] && echo "  Untracked:" && echo "$UNTRACKED" | sed 's/^/    /'
    echo ""
    read -rp "Commit ALL changes before releasing? [y/N] " CONFIRM
    if [[ "$CONFIRM" =~ ^[Yy]$ ]]; then
        git add -A
        git commit -m "chore: pending changes before release ${TAG}"
    else
        die "Release aborted. Commit or stash your changes first."
    fi
fi

# ── Check tag doesn't already exist ──
if git tag -l "$TAG" | grep -q .; then
    die "Tag ${TAG} already exists. Bump to a new version."
fi

if git branch -l "$RELEASE_BRANCH" | grep -q .; then
    die "Branch ${RELEASE_BRANCH} already exists. Delete it first or use a new version."
fi

# ── Bump versions ──
info "Bumping version to ${VERSION}"

CARGO_FILES=$(find crates -name Cargo.toml -not -path "*/target/*")
for f in $CARGO_FILES; do
    sed -i -E "s/^version *= *\"[0-9.]+\"/version = \"${VERSION}\"/" "$f"
    sed -i -E "s/^version *= *'[0-9.]+'/version = '${VERSION}'/" "$f"
done

sed -i -E 's/"version": "[0-9.]+"/"version": "'"${VERSION}"'"/' web/package.json

sed -i "s/version: 'v[0-9.]*'/version: 'v${VERSION}'/" web/src/test/server.ts

info "Verifying builds compile"
cargo check -q 2>&1 || die "Rust build failed. Fix before releasing."

info "Version bump complete. Changes:"
git diff --stat

echo ""
read -rp "Proceed with release ${TAG}? [y/N] " CONFIRM
[[ "$CONFIRM" =~ ^[Yy]$ ]] || die "Aborted."

# ── Commit version bump ──
info "Committing version bump"
git add -A
git commit -m "chore: bump version to ${VERSION}"

# ── Create release branch, merge to main, tag ──
info "Creating release branch ${RELEASE_BRANCH}"
git branch "$RELEASE_BRANCH" develop

info "Switching to main"
git checkout main

info "Merging ${RELEASE_BRANCH} into main"
git merge "$RELEASE_BRANCH"

info "Creating tag ${TAG}"
git tag "$TAG"

# ── Switch back to develop ──
info "Switching back to develop"
git checkout develop

# ── Push everything ──
info "Pushing to origin"
git push origin main
git push origin "$TAG"
git push origin develop
git push origin "$RELEASE_BRANCH"

echo ""
info "Release ${TAG} complete!"
echo "  main:         $(git rev-parse --short main)"
echo "  tag:          ${TAG}"
echo "  develop:      $(git rev-parse --short develop)"
echo ""
info "CI will build binaries and Docker image."
info "Deploy with: docker pull ghcr.io/chenxiaoli/llm-gateway:${VERSION}"

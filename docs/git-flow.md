# Git Flow Best Practices

Based on [Vincent Driessen's branching model](https://nvie.com/posts/a-successful-git-branching-model/).

## Branches

| Branch | Purpose | Branches from | Merges into |
|--------|---------|---------------|-------------|
| `main` | Production-ready code only | — | — |
| `develop` | Integration branch for next release | — | — |
| `feature/*` | New feature development | `develop` | `develop` |
| `release/*` | Release stabilization | `develop` | `main` + `develop` |
| `hotfix/*` | Urgent production fix | `main` | `main` + `develop` |

## Feature Workflow

```bash
git checkout develop
git checkout -b feature/my-feature
# ... develop ...
git checkout develop
git merge --no-ff feature/my-feature
git branch -d feature/my-feature
```

## Release Workflow

### 1. Create release branch from develop

```bash
git checkout develop
git checkout -b release/0.6.1
```

### 2. Version bump

Update version in all config files:

- `crates/*/Cargo.toml` (9 files) — `version = "x.y.z"`
- `web/package.json` — `"version": "x.y.z"`

Regenerate lockfiles:

```bash
cargo build --workspace        # updates Cargo.lock
cd web && npm install           # updates package-lock.json
```

### 3. Update CHANGELOG.md

Document changes under the new version heading. Follow [Keep a Changelog](https://keepachangelog.com/) format:

```markdown
## [0.6.1] - 2026-04-26

### Added
- ...

### Changed
- ...

### Fixed
- ...
```

### 4. Final testing

```bash
cargo test --workspace          # backend tests
cd web && npm test              # frontend tests
```

### 5. Finish release

Merge to `main` with tag, then merge back to `develop`:

```bash
git checkout main
git merge --no-ff release/x.y.z -m "Release x.y.z"
git tag -a vx.y.z -m "Release vx.y.z"

git checkout develop
git merge --no-ff release/x.y.z -m "Merge release/x.y.z into develop"

git branch -d release/x.y.z
git push origin main develop --tags
```

**Always use `--no-ff`** to preserve release topology in git history.

## Hotfix Workflow

```bash
git checkout main
git checkout -b hotfix/0.6.2
# bump patch version, fix bug
git checkout main
git merge --no-ff hotfix/0.6.2 -m "Hotfix 0.6.2"
git tag -a v0.6.2 -m "Hotfix v0.6.2"

git checkout develop
git merge --no-ff hotfix/0.6.2 -m "Merge hotfix/0.6.2 into develop"

git branch -d hotfix/0.6.2
git push origin main develop --tags
```

## Rules

- **No direct commits** to `main`
- **No new features** on `release/*` — only version bump, changelog, bug fixes
- **Always `--no-ff`** for release and hotfix merges
- **Short-lived branches** — release branches should not live more than 1-2 weeks
- **Annotated tags** — `git tag -a` with descriptive message

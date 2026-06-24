# Ticket 82 — `cocogitto` for conventional commits + version sync

**Phase:** 9 (release plumbing)
**Depends on:** [81](./81-tauri-action-release.md)
**Blocks:** none

## Goal

Wire [`cocogitto`](https://docs.cocogitto.io/) (a single Rust binary, installed via `mise`) as the source of truth for:

1. **Commit grammar** — every commit on `main` must be a [Conventional Commit](https://www.conventionalcommits.org/). Enforced by a prek hook.
2. **Version bumps** — `cog bump --auto` walks commits since the last tag, picks the right semver bump, atomically updates four version fields (`package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `sidecar/pyproject.toml`), regenerates `CHANGELOG.md`, commits, and tags. The tag triggers Ticket 81's release workflow.

## Why `cocogitto`, not commitizen / release-please / semantic-release

- **Self-contained**: one binary, installable via `mise`. No npm/python at release time.
- **Knows multi-file version sync** out of the box via `[bump_profiles]` regex replacement — exactly the four-file problem we have.
- **Composes with `just`** — `just release` is one line.
- **Local, inspectable**: you see the diff before tagging. Compare to `release-please` which lives in CI and opens a release PR; that's also fine, just a different ergonomic.

If we ever want the fully-automated "merge to main → release PR opens" flow, we add `release-please` as a layer *on top* of `cocogitto` without breaking the local path.

## Files

- Modify: `.mise.toml` — pin `cocogitto = "latest"`.
- Create: `cog.toml` (at repo root).
- Create: `.cog/templates/CHANGELOG.tera` — custom Tera template if the default needs tweaking; otherwise omit.
- Modify: `justfile` — `just release <patch|minor|major>` recipe.
- Modify: `.pre-commit-config.yaml` — `cog verify` hook on commit-msg stage.
- Modify: `CLAUDE.md` — short section on commit format (link to https://www.conventionalcommits.org/).

## Steps

- [ ] **1. Pin the binary.**

  ```toml
  # .mise.toml
  [tools]
  cocogitto = "latest"
  ```

  ```sh
  mise install
  cog --version  # must succeed
  ```

- [ ] **2. `cog.toml`.**

  ```toml
  # https://docs.cocogitto.io/config/index.html
  from_latest_tag = true
  ignore_merge_commits = true
  branch_whitelist = ["main"]
  tag_prefix = "v"

  # Commit footers / scopes you actually want.
  [commit_types]

  # Bump the same version into every artefact that ships a version string.
  [bump_profiles]

  [[packages]]
  path = "package.json"
  bump_profile = "default"
  [[packages.bump_hooks]]
  hook = "sed -E -i.bak 's/(\"version\": \")[^\"]*(\")/\\1{{version}}\\2/' package.json && rm package.json.bak"

  [[packages]]
  path = "src-tauri/Cargo.toml"
  [[packages.bump_hooks]]
  hook = "sed -E -i.bak 's/^version = \"[^\"]*\"/version = \"{{version}}\"/' src-tauri/Cargo.toml && rm src-tauri/Cargo.toml.bak"

  [[packages]]
  path = "src-tauri/tauri.conf.json"
  [[packages.bump_hooks]]
  hook = "sed -E -i.bak 's/(\"version\": \")[^\"]*(\")/\\1{{version}}\\2/' src-tauri/tauri.conf.json && rm src-tauri/tauri.conf.json.bak"

  [[packages]]
  path = "sidecar/pyproject.toml"
  [[packages.bump_hooks]]
  hook = "sed -E -i.bak 's/^version = \"[^\"]*\"/version = \"{{version}}\"/' sidecar/pyproject.toml && rm sidecar/pyproject.toml.bak"

  [pre_bump_hooks]
  # Re-run the lint suite before we ever cut a tag. CI also runs this — belt + braces.
  hooks = ["just lint", "just test"]

  [post_bump_hooks]
  hooks = []
  ```

  Implementer note: confirm `[bump_profiles]` syntax against the installed `cog` version. Cocogitto's TOML schema has shifted across releases; pin the version, copy from current docs.

- [ ] **3. `just release` recipe.**

  ```make
  # justfile
  release type='auto':
      mise exec -- cog bump --{{type}}
      git push --follow-tags
  ```

  `just release auto` (the default) → cog picks the bump from commits.
  `just release patch` / `minor` / `major` → forced.

- [ ] **4. Commit-msg hook in prek.**

  ```yaml
  # .pre-commit-config.yaml
  - repo: local
    hooks:
      - id: cog-verify
        name: 'cog verify (conventional commits)'
        language: system
        entry: mise exec -- cog verify --file
        stages: [commit-msg]
  ```

  Reinstall hooks so commit-msg-stage hooks land:

  ```sh
  prek install --install-hooks --hook-type pre-commit --hook-type commit-msg
  ```

  Update `just hooks-install` accordingly.

- [ ] **5. Document in CLAUDE.md.**

  Append a short "Commit format" section:

  ```md
  Commits on `main` are Conventional Commits — enforced by the `cog verify`
  prek hook. The summary line drives the version bump at release time
  (Ticket 82). `feat:` → minor, `fix:` → patch, `feat!:` / `BREAKING CHANGE:`
  footer → major, others (`chore:`, `docs:`, `refactor:`, ...) → no bump.
  ```

- [ ] **6. Smoke test.**

  ```sh
  # On a throwaway branch:
  git commit --allow-empty -m "feat: smoke test cog bump"
  just release auto
  # Inspect: package.json / Cargo.toml / tauri.conf.json / pyproject.toml all bumped,
  # CHANGELOG.md updated, tag v0.x.0 created.
  # Roll back: git tag -d v0.x.0 && git reset --hard HEAD~2
  ```

- [ ] **7. Commit.**

  ```sh
  git commit -m "feat(release): cocogitto for conventional commits + version sync"
  ```

## Acceptance

- `just release auto` bumps all four version fields in one commit + tag.
- The `cog verify` commit-msg hook rejects a commit that is not Conventional.
- `CHANGELOG.md` regenerates with the new entries grouped by type.
- Tag push triggers Ticket 81's release workflow successfully.

## Out of scope

- Per-component versioning (independent sidecar vs frontend version) — overkill for MVP. Single version across the workspace.
- `release-please` automation layer — fast-follow once manual cuts feel like toil.
- Custom CHANGELOG template — accept the default unless it reads poorly.

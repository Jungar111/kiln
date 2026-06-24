# Ticket 81 — Tag-triggered release workflow via `tauri-action`

**Phase:** 9 (release plumbing)
**Depends on:** [80](./80-pyinstaller-sidecar.md)
**Blocks:** [82](./82-cocogitto-versioning.md)

## Goal

A GitHub Actions workflow that, on tag push (`v*.*.*`), builds the desktop app on macOS-arm64, macOS-x86_64, windows-latest, and ubuntu-latest, builds the PyInstaller sidecar in the same job, signs the macOS / Windows artefacts, and attaches the bundles to a GitHub release.

This is the equivalent of `goreleaser` for the Tauri stack — `tauri-action` is the project-blessed entry point.

## Files

- Create: `.github/workflows/release.yml`.
- Modify: `src-tauri/tauri.conf.json` — fill in `bundle.macOS.signingIdentity`, `bundle.windows.signCommand`, updater config (deferred — placeholder commit referencing the keys).
- Create: `docs/release.md` — operator runbook (keys, secrets, how to cut a release).

## Steps

- [ ] **1. Workflow skeleton.**

  ```yaml
  name: release
  on:
    push:
      tags: ["v*.*.*"]

  jobs:
    release:
      strategy:
        fail-fast: false
        matrix:
          include:
            - { platform: macos-latest,   target: aarch64-apple-darwin       }
            - { platform: macos-13,       target: x86_64-apple-darwin        }
            - { platform: ubuntu-22.04,   target: x86_64-unknown-linux-gnu   }
            - { platform: windows-latest, target: x86_64-pc-windows-msvc     }
      runs-on: ${{ matrix.platform }}
      steps:
        - uses: actions/checkout@v4
        - uses: jdx/mise-action@v2
        - run: just bootstrap
        - run: just build-sidecar
        - uses: tauri-apps/tauri-action@v0
          env:
            GITHUB_TOKEN:               ${{ secrets.GITHUB_TOKEN }}
            APPLE_CERTIFICATE:          ${{ secrets.APPLE_CERTIFICATE }}
            APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
            APPLE_SIGNING_IDENTITY:     ${{ secrets.APPLE_SIGNING_IDENTITY }}
            APPLE_ID:                   ${{ secrets.APPLE_ID }}
            APPLE_PASSWORD:             ${{ secrets.APPLE_PASSWORD }}
            APPLE_TEAM_ID:              ${{ secrets.APPLE_TEAM_ID }}
            TAURI_SIGNING_PRIVATE_KEY:  ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
            TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
          with:
            args: --target ${{ matrix.target }}
            tagName: ${{ github.ref_name }}
            releaseName: Kiln ${{ github.ref_name }}
            releaseBody: "See CHANGELOG.md."
            releaseDraft: true
            prerelease: false
  ```

  We use `mise-action` so CI installs the exact toolchain pinned in `.mise.toml` — no drift between local and release.

- [ ] **2. Secrets.** Document in `docs/release.md` exactly which secrets are required and how to generate each. The set:

  - `APPLE_CERTIFICATE` / `APPLE_CERTIFICATE_PASSWORD` — base64-encoded `.p12` Apple Developer ID Application cert.
  - `APPLE_ID` / `APPLE_PASSWORD` / `APPLE_TEAM_ID` — for notarization.
  - `TAURI_SIGNING_PRIVATE_KEY` / `..._PASSWORD` — Tauri updater key pair (generated via `pnpm tauri signer generate`).
  - Windows signing — deferred. Document the path (Azure Trusted Signing or an OV cert) but do not block this ticket on it; ship un-signed `.msi` initially with a `releaseDraft: true` gate.

- [ ] **3. Updater config.** In `tauri.conf.json` add a placeholder `plugins.updater.endpoints` pointing at a `latest.json` hosted on the GitHub release. Real updater rollout is its own follow-up — this ticket just leaves the seam open.

- [ ] **4. Runbook.**

  ```md
  <!-- docs/release.md -->
  # Cutting a release

  1. Land all in-flight PRs on `main`.
  2. `just release patch` (Ticket 82) — bumps versions, regenerates CHANGELOG, commits, tags.
  3. `git push --follow-tags`.
  4. GitHub Actions runs `.github/workflows/release.yml`. Watch it; if it goes green, a **draft** release is created with the bundles attached.
  5. Verify each bundle on the relevant OS (macOS notarization can take 5-15 min).
  6. Mark the draft as published.
  ```

- [ ] **5. Sanity-trigger.** Tag `v0.0.0` (no real code change), let the workflow run, confirm artefacts land.

- [ ] **6. Commit.**

  ```sh
  git commit -m "feat(release): tag-triggered tauri-action release workflow"
  ```

## Acceptance

- A test tag produces four platform bundles (macOS-arm64, macOS-x86_64, windows-x86_64, linux-x86_64) attached to a draft GitHub release.
- The macOS-arm64 bundle launches and opens without quarantine warnings *after* notarization.
- The sidecar binary inside the bundle is signed-by-association (verified via `codesign --verify --deep`).

## Out of scope

- Windows code signing — deferred; document the gap in `docs/release.md`.
- Auto-update rollout (the actual `latest.json` publish) — fast-follow ticket.
- Linux package signing / repo distribution (snap, flatpak) — out of MVP.

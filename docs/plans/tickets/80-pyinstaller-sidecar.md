# Ticket 80 — Bundle the Python sidecar with PyInstaller

**Phase:** 9 (release plumbing)
**Depends on:** all of phases 1-8 (sidecar code must be feature-complete enough to ship)
**Blocks:** [81](./81-tauri-action-release.md)

## Goal

Produce a single platform-specific executable per OS that contains the `kiln-sidecar` script + every Python dependency (no system Python required on the user's machine). Declare the resulting binary as a Tauri `externalBin` so `tauri build` ships it inside the `.app` / `.exe` / `.AppImage`.

> Spec §6 — *one sidecar per project*. Production users do not have `uv` installed; the sidecar must be self-contained.

## Why PyInstaller, not PyOxidizer / shiv / nuitka

- **PyInstaller** is boring, battle-tested, and supports macOS-universal + Windows + Linux. The size cost (~30-60 MB per platform) is acceptable for a desktop app.
- PyOxidizer is faster at startup but produces fragile builds with native C extensions — `pyarrow` and `mlflow` both ship native code, so this gets brittle.
- shiv assumes a system Python is present.
- nuitka compiles, but compile-time on `pyarrow`-class deps is painful in CI.

If PyInstaller binary size becomes the user-visible problem (>200 MB), reach for PyOxidizer in a follow-up — *not* in this ticket.

## Files

- Create: `sidecar/pyinstaller.spec` — single source of truth for the build.
- Create: `sidecar/build_binary.py` — small helper that calls PyInstaller and copies the output to a stable path (`src-tauri/binaries/kiln-sidecar-<triple>`).
- Modify: `sidecar/pyproject.toml` `[dependency-groups] dev` — add `pyinstaller>=6`.
- Modify: `src-tauri/tauri.conf.json` — `bundle.externalBin: ["binaries/kiln-sidecar"]`.
- Modify: `src-tauri/src/sidecar.rs` — spawn via `tauri::process::Command::new_sidecar("kiln-sidecar")` in release builds, fall back to `uv run` in dev.
- Modify: `justfile` — `just build-sidecar` recipe.
- Modify: `.gitignore` — exclude `sidecar/build/`, `sidecar/dist/`.

## Steps

- [ ] **1. Write the spec.**

  ```python
  # sidecar/pyinstaller.spec — driven by `pyinstaller pyinstaller.spec`
  from PyInstaller.utils.hooks import collect_all

  hidden = []
  datas = []
  binaries = []
  for pkg in ("mlflow", "pyarrow", "ipykernel", "jupyter_client"):
      _datas, _binaries, _hidden = collect_all(pkg)
      datas += _datas
      binaries += _binaries
      hidden += _hidden

  a = Analysis(
      ["src/kiln_sidecar/__main__.py"],
      pathex=["src"],
      binaries=binaries,
      datas=datas,
      hiddenimports=hidden,
      hookspath=[],
      runtime_hooks=[],
      excludes=["tkinter", "matplotlib.tests", "pytest"],
      noarchive=False,
  )
  pyz = PYZ(a.pure)
  exe = EXE(
      pyz, a.scripts,
      name="kiln-sidecar",
      console=True,
      onefile=True,
      strip=False,
      upx=False,
  )
  ```

- [ ] **2. Build helper.**

  ```python
  # sidecar/build_binary.py
  from __future__ import annotations

  import platform
  import shutil
  import subprocess
  import sys
  from pathlib import Path

  TRIPLE_MAP: dict[tuple[str, str], str] = {
      ("Darwin", "arm64"): "aarch64-apple-darwin",
      ("Darwin", "x86_64"): "x86_64-apple-darwin",
      ("Linux", "x86_64"): "x86_64-unknown-linux-gnu",
      ("Linux", "aarch64"): "aarch64-unknown-linux-gnu",
      ("Windows", "AMD64"): "x86_64-pc-windows-msvc",
  }


  def main() -> int:
      sidecar_dir = Path(__file__).parent
      subprocess.run(
          ["pyinstaller", "pyinstaller.spec", "--noconfirm", "--clean"],
          cwd=sidecar_dir,
          check=True,
      )
      triple = TRIPLE_MAP.get((platform.system(), platform.machine()))
      if triple is None:
          print(f"unsupported platform: {platform.system()} {platform.machine()}", file=sys.stderr)
          return 1
      out_dir = sidecar_dir.parent / "src-tauri" / "binaries"
      out_dir.mkdir(parents=True, exist_ok=True)
      ext = ".exe" if platform.system() == "Windows" else ""
      src = sidecar_dir / "dist" / f"kiln-sidecar{ext}"
      dst = out_dir / f"kiln-sidecar-{triple}{ext}"
      shutil.copy2(src, dst)
      print(f"wrote {dst}")
      return 0


  if __name__ == "__main__":
      raise SystemExit(main())
  ```

  Tauri matches `externalBin: "binaries/kiln-sidecar"` against `binaries/kiln-sidecar-<host-triple>`; the naming above is exactly what Tauri's bundler expects.

- [ ] **3. Wire `tauri.conf.json`.**

  ```jsonc
  "bundle": {
    "active": true,
    "targets": "all",
    "externalBin": ["binaries/kiln-sidecar"],
    ...
  }
  ```

- [ ] **4. Spawn path.** In `src-tauri/src/sidecar.rs`, switch between the dev path (`uv run --directory sidecar kiln-sidecar`) and the bundled path:

  ```rust
  #[cfg(debug_assertions)]
  fn sidecar_command(repo_root: &std::path::Path) -> tokio::process::Command {
      let mut cmd = tokio::process::Command::new("uv");
      cmd.arg("run").arg("--directory").arg(repo_root.join("sidecar")).arg("kiln-sidecar");
      cmd
  }

  #[cfg(not(debug_assertions))]
  fn sidecar_command(_: &std::path::Path) -> tokio::process::Command {
      // tauri_plugin_shell exposes the sidecar via app handle in release; this
      // free function takes the path resolved by the shell plugin instead.
      // The implementer should follow tauri's `process::CommandChild::new_sidecar`
      // shape and pass through the resolved path.
      unimplemented!("wired by tauri shell plugin in release build")
  }
  ```

  Implementer note: the cleanest release path is to use `tauri-plugin-shell`'s `Command::new_sidecar`. That requires moving the spawn call into Tauri's setup hook with access to the `AppHandle`. Plan to refactor `Sidecar::spawn(&Path)` → `Sidecar::spawn(&AppHandle)` in this ticket.

- [ ] **5. `just build-sidecar`.**

  ```make
  build-sidecar:
      cd sidecar && uv run python build_binary.py
  ```

  Update `just build-tauri` to depend on it.

- [ ] **6. Smoke test (host platform only).**

  ```sh
  just build-sidecar
  ls src-tauri/binaries/
  just build-tauri
  ```

  Open the produced bundle. Confirm the app launches the bundled sidecar (check `ps aux | grep kiln-sidecar` — should show the bundled binary, not `python`).

- [ ] **7. Commit.**

  ```sh
  git commit -m "feat(release): bundle sidecar with PyInstaller as Tauri externalBin"
  ```

## Acceptance

- A built `.app` / `.exe` on the host platform runs end-to-end without `uv` / Python installed.
- `src-tauri/binaries/` is gitignored (the file is rebuilt in CI) — verify.
- Bundle size is logged; flag if > 200 MB.

## Out of scope

- Cross-compilation. The matrix in Ticket 81 builds each platform on its native runner.
- Code signing for the sidecar binary (it inherits the parent bundle's signature via Tauri) — verify, but no separate signing here.
- Bundle-size reductions (UPX, tree-shaking) — fast-follow when a number is justified.

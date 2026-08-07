# Luna Studio Development Log

This file records repository-level operations so another model or developer can continue the work without reconstructing the local state.

## 2026-08-07 - GitHub source repository preparation

- Confirmed the active project root is `F:\Insta360onWin`; no project files were created or modified on the C drive.
- Confirmed the directory was not previously initialized as a Git repository and had no remote.
- Audited the project tree, large files, nested repositories, build outputs and common secret patterns.
- Found no API keys, access tokens or private keys in `src`, `web` or `data` using the repository scan patterns.
- Expanded `.gitignore` to exclude Rust build directories, packaged EXE/DLL files, WebView2 runtime data, gallery thumbnail cache and editor/OS cache files.
- Excluded the bundled `assets/ffmpeg/ffmpeg.exe` binary and added `assets/ffmpeg/README.md` documenting the expected runtime path.
- Excluded generated or extracted APK artifacts including DEX/SO files, heap dumps, reconstructed DEX files, validation outputs and nested reference repositories.
- Kept application source, the HTML interface, watermark resources, `data/profiles.json`, protocol analysis scripts, compact PCAP findings and handoff documents in the source set.
- Added `.gitattributes` to make text and binary handling deterministic across Windows and macOS development machines.
- Created the private GitHub repository under the authenticated account. The initially created `Insta360onWin` repository was renamed to `Insta360Linker` before the first push at the user's request.
- Initialized a local Git repository with `main` as the default branch and enabled CRLF working-tree normalization.
- Staged 95 source/resource/document files totaling approximately 3.68 MB; no staged file approaches GitHub's per-file size limit.
- Re-ran the staged credential scan; no GitHub token, private-key, API-key or access-token patterns were found.
- Ran `cargo check --locked --bin html_app`; it completed successfully with existing dead-code warnings only.
- Final GitHub repository: `https://github.com/wuhaiqi0621/Insta360Linker`.
- Added the GitHub repository as the local `origin`, pushed the initial source import commit `db60341` to `main`, and configured the local branch to track `origin/main`.

## 2026-08-07 - Outer-frame watermark font correction

- Imported the user-provided `G:\备份\字体\BeihaibeiSC-Regular.ttf` into `assets/apk_watermark/BeihaibeiSC-Regular.ttf` without modifying the source file.
- Verified the copied font with SHA-256 `1240EC12D92EF9A30DF6BC473BF37D8F213A6E6ABF1C4CE132E30DEFF44C0B5D`.
- Changed only the outer-frame metadata renderer to use Beihaibei SC for aperture, shutter speed, ISO and timestamp text.
- Kept the application UI font, top branding artwork and `luna moment` signature artwork unchanged.
- Ran the HTML application's watermark test suite: 15 tests passed, 0 failed and the external-image QA test remained explicitly ignored in the regular suite.
- Ran the ignored external-image QA test against `G:\Insta360 Luna Ultra\IMG_20260626_164557_021.jpg`; it passed and rendered `target/beihaibei-frame-preview.png` with shutter speed `1/800`.
- Built the release `html_app` successfully and updated the local daily executable `F:\Insta360onWin\LunaStudio.exe` (SHA-256 `A1541AD7758058E6EE1B2C3FF77D716C34263220D2ED132FAD81D0D894BAABE4`).

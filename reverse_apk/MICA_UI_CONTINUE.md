# Mica / UI Continuation Log

Project root: `F:/Insta360onWin`

Daily entry point: `F:/Insta360onWin/run_release.bat`

Current executable: `F:/Insta360onWin/LunaStudio.exe`

Primary files:
- `src/bin/html_app.rs`: tao window, DWM attributes, transparent wry WebView2, native IPC.
- `web/index.html`: complete Chinese daily UI, Mica-aware CSS, gallery and camera interactions.
- `reverse_apk/REVERSE_CONTINUE_LOG.md`: full protocol and historical implementation log.

Rules for continued work:
- Keep all generated files and build targets on the F drive.
- Keep source and documentation UTF-8 without BOM and CRLF.
- Do not terminate user-owned app processes.
- Record each implementation and validation stage below before handing off.

## Operation 1 - Started an independent Mica/UI audit

User request:
- Finish native Mica integration.
- Adapt or reset the HTML UI for Mica.
- Check the previous partial implementation for defects.
- Keep a detailed Markdown handoff for another model.

Audit scope:
- DWM capability/result handling and unsupported-system fallback.
- tao top-level transparency and wry WebView2 transparency.
- HTML root transparency, surface hierarchy, dark/light modes, reduced transparency, and forced colors.
- Narrow-window overflow and gallery rendering cost.
- Release build, visual smoke test, cleanup, and final hash.

Status: completed by Operations 2 through 6 below.

## Operation 2 - Audited the previous Mica implementation

Environment observed:
- Windows 11 Pro, build 26200.
- App theme is dark (`AppsUseLightTheme=0`).
- Windows transparency effects are enabled (`EnableTransparency=1`).
- No LunaStudio process was running during the audit.

Confirmed working:
- tao top-level window uses `with_transparent(true)`.
- wry WebView2 uses `with_transparent(true)`.
- HTML and body backgrounds are transparent.
- Native code requests the main-window system backdrop and has an older Windows 11 Mica attribute fallback.
- Theme changes reapply native DWM attributes.
- HTML has light, dark, and reduced-transparency palettes.

Defects found:
1. `apply_mica` discards both DWM HRESULT values, so the HTML cannot switch to an opaque fallback when Mica is unsupported.
2. There is no `forced-colors` fallback for Windows High Contrast mode.
3. Every visible video badge still uses `backdrop-filter`, creating avoidable per-card GPU work in large galleries.
4. The historical log claims blur is limited to fixed chrome, but the CSS does not fully match that claim.

Decision:
- Keep the current information architecture; it is already a usable daily UI and does not need a destructive reset.
- Fix the native capability handoff and accessibility/performance gaps, then repeat visual checks at desktop and minimum window sizes.

Status: audit complete; fixes in progress.

## Operation 3 - Fixed native capability handoff and frontend fallbacks

Native changes in `src/bin/html_app.rs`:
- Changed `apply_mica` to return whether the main backdrop or legacy Mica call succeeded.
- Replaces the embedded HTML root with `class="native-mica"` or `class="no-native-mica"` before WebView2 loads it.
- Kept DWM corner and title-bar theme updates non-fatal.
- Theme-change handling now reapplies DWM attributes without changing event-loop return types.

Frontend changes in `web/index.html`:
- Added a fully opaque light/dark palette for `html.no-native-mica`.
- Disabled backdrop filters when native Mica is unavailable.
- Expanded reduced-transparency handling to the context menu and live-status overlay.
- Added Windows High Contrast support through `@media (forced-colors: active)` and system color keywords.
- Removed `backdrop-filter` from every video badge; badges now use a slightly stronger solid translucent fill.
- Kept blur only on a few fixed/temporary surfaces: sidebar, top bar, one live-status overlay, and one context menu.

Status: implementation complete; validated by Operations 4 and 5 below.

## Operation 4 - Repeated visual validation and refined the Mica hierarchy

Validation observations:
- The first desktop-size capture was invalid because another window covered the app; it was discarded and repeated with Luna Studio explicitly brought to the foreground.
- The clean desktop capture showed no horizontal overflow at the default window size.
- The minimum-size capture at 760 x 560 showed the collapsed sidebar, wrapped connection controls, vertical scrolling, and no horizontal clipping.
- Native Mica was visible, but the main workspace was too transparent in dark mode and appeared washed out against a light desktop backdrop.

Frontend refinement:
- Increased the opacity of the workspace, content surfaces, and strong surfaces in both light and dark palettes.
- Retained visible native material through the page background, sidebar, top bar, and spacing between surfaces.
- Kept blur limited to fixed or temporary chrome so gallery scrolling does not create one compositor filter per card.

Status: palette refinement complete; rebuilt and visually validated by Operation 5.

## Operation 5 - Rebuilt, tested, and visually validated the final UI

Automated checks:
- Inline JavaScript syntax check passed with Node.js.
- `cargo fmt --all -- --check` passed.
- `cargo test --bin html_app` passed: 10 tests, 0 failures.
- `cargo build --release --bin html_app` completed successfully.
- Existing dead-code warnings in the unused OSC adapter remain warnings only; this task did not expand or suppress them.
- UTF-8 without BOM and CRLF were verified for the touched Rust, HTML, and Markdown files.

Visual checks on the rebuilt executable:
- Default window capture: 1196 x 819 including native frame; no horizontal overflow or clipped controls.
- Minimum window capture: 760 x 560; collapsed navigation, connection controls, camera console, and vertical scrollbar remained usable.
- Dark-mode surface opacity no longer allows a light desktop backdrop to wash out the main controls.
- Live preview and media viewing areas remain opaque black/dark for accurate image and video display.

Validation note:
- The first formatting/encoding verification command had a PowerShell output-pipeline syntax error and stopped before executing; it was corrected and rerun successfully.
- All compiler targets, temporary files, screenshots, and .NET compiler temp paths used for this audit were placed on the F drive.

Status: final build validated.

## Operation 6 - Published the daily build and removed audit artifacts

Release:
- Replaced `F:/Insta360onWin/LunaStudio.exe` with the validated release build.
- Hidden launch smoke test created the native window successfully and exited cleanly.
- File size: 8,806,400 bytes.
- SHA-256: `F058AB915780073BB0BF1F55987318A1374BF943EC2004F304031F7FB807077F`.

Cleanup:
- Removed `LunaStudio-audit.exe` and its WebView2 profile.
- Removed `target_mica_audit`.
- Removed the temporary Mica validation scripts and screenshots.
- Confirmed that `LunaStudio.exe` is the only application executable in the project root.
- No project process was left running.

Current Mica behavior:
- Windows 11 uses `DWMWA_SYSTEMBACKDROP_TYPE = DWMSBT_MAINWINDOW`, with the legacy Mica attribute as fallback.
- The HTML receives `native-mica` only when one of those DWM calls succeeds.
- Unsupported systems receive `no-native-mica` and a fully opaque light/dark palette.
- Reduced transparency and Windows High Contrast both disable frontend blur and use accessible opaque/system colors.

Status: complete. The next model should start from `LunaStudio.exe`, this file, and `REVERSE_CONTINUE_LOG.md`; no unfinished Mica/UI action remains.

## Operation 7 - Audited the original AstroBox implementation after user feedback

User feedback:
- The published build did not show an obvious Mica effect.
- The requested reference is the original AstroBox project, not AstroBox-NG or next-gen.

Reference source:
- Repository: `AstralSightStudios/AstroBox-Public`.
- Audited main commit: `616c5dbcc6653b8c124337122396d18226bfbd8c`.
- Local read-only reference: `reverse_apk/references/AstroBox-Public`.
- Its `src-tauri/tauri.conf.json` sets both `transparent: true` and `windowEffects.effects: ["mica"]`.
- Its lockfile resolves `window-vibrancy 0.6.0`; that crate applies `DWMWA_SYSTEMBACKDROP_TYPE = DWMSBT_MAINWINDOW` on current Windows 11 and attribute 1029 on early Windows 11.
- This is the same native DWM effect requested by Luna Studio, so replacing the whole window framework is unnecessary.

Root cause found:
- Luna Studio applied the DWM attribute before creating WebView2 and did not reapply it immediately afterward.
- More importantly, the last palette refinement covered the native material with a 72% dark page overlay, 76% top-bar overlay, and 90% sidebar overlay.
- AstroBox keeps its root and desktop navigation transparent; in dark mode its main content layer is only `rgba(255, 255, 255, 0.04)`.
- Therefore the Luna Studio DWM call could succeed while the frontend made the material visually indistinguishable from a flat dark background.

Implementation started:
- Reapply Mica after WebView2 creation and synchronize the HTML `native-mica`/`no-native-mica` class with that result.
- Synchronize the same class again after Windows theme changes.
- For `native-mica`, make the app root transparent and use AstroBox-style light material overlays instead of an opaque dark sheet.
- Keep high-opacity surfaces only for fields, capture controls, dialogs, and media viewing where readability or image accuracy requires them.
- Preserve the existing opaque unsupported-system, reduced-transparency, and High Contrast fallbacks.

Status: implementation complete; build, DWM-state query, and visual validation pending.

## Operation 8 - Verified native DWM state and corrected real-backdrop contrast

Native verification on the rebuilt executable:
- `DwmGetWindowAttribute(hwnd, 38, ...)` returned HRESULT `0` and value `2`.
- Value `2` is `DWMSBT_MAINWINDOW`, confirming that native Mica was active after WebView2 creation.
- `DwmGetWindowAttribute(hwnd, 20, ...)` returned HRESULT `0` and value `1`, confirming the immersive dark-mode window attribute.

First screenshot result:
- The root material became clearly visible at both 1196 x 819 and 760 x 560.
- On this Windows configuration the transparent client-area material remained much lighter than the title bar even though dark mode was enabled.
- The first AstroBox-style 3.5% light overlay therefore produced insufficient contrast for the dark-palette foreground text and was rejected rather than published.

Contrast correction:
- Added a 55% dark material tint to the native-Mica dark workspace.
- Main cards use a 62% dark tint, strong controls use 78%, and fixed chrome/sidebar use 52%/66%.
- These values remain materially more transparent than the rejected previous release, which used 72% for the whole page, 76% for the top bar, 82% to 94% for surfaces, and 90% for the sidebar.
- Light mode still follows the original AstroBox pattern: transparent root, lightly tinted chrome, and mostly white content surfaces.

Status: contrast correction complete; rebuilt and validated by Operation 9.

## Operation 9 - Published the AstroBox-aligned Mica build

Final validation:
- Inline JavaScript syntax check passed.
- `cargo fmt --all -- --check` passed.
- `cargo test --bin html_app`: 10 passed, 0 failed.
- Release build completed successfully with only the pre-existing unused OSC/dead-code warnings.
- Final screenshots passed at 1196 x 819 and 760 x 560 with readable text, intact camera controls, vertical scrolling, and no horizontal overflow.
- Final audit executable returned DWM backdrop HRESULT `0`, type `2`.
- The published `LunaStudio.exe` also returned DWM backdrop HRESULT `0`, type `2` during its hidden smoke test.

Release:
- File: `F:/Insta360onWin/LunaStudio.exe`.
- Size: 8,807,936 bytes.
- SHA-256: `934D9917B20EFD5CBC84D8751E50517C561541E48A36841C55C6DEB4B029142C`.

Cleanup and retained evidence:
- Removed `LunaStudio-astrobox-audit.exe`, its WebView2 profile, `target_mica_astrobox`, and all temporary validation scripts/screenshots.
- Kept `reverse_apk/references/AstroBox-Public` at commit `616c5dbc` as the user-requested original AstroBox reference.
- Kept `reverse_apk/references/window-vibrancy-0.6.0` at tag commit `52692649` to preserve the exact native implementation resolved by AstroBox's lockfile.
- No test or application process was left running.

Status: complete. The active daily build now has a measured native MainWindow Mica backdrop and a frontend that leaves the material visible instead of covering it with the previous near-opaque palette.

## Operation 10 - Corrected the inaccurate root-transparency claim

User verification:
- The user still could not see Mica and asked whether the frontend root was actually transparent.
- Re-reading the final CSS proved the concern valid: dark `html.native-mica` set `--bg: rgba(9, 14, 20, .55)`, and `.app` painted that variable over its full area.
- HTML and body were transparent, but the full-size `.app` element was not. The previous release therefore did not meet the stated “truly transparent root” condition.

Additional original-AstroBox audit:
- Added a read-only sparse source snapshot of Tauri 2.7.0 at `reverse_apk/references/tauri-2.7.0`, commit `96439c2c`.
- Tauri attaches the detached WebView first, then calls `set_window_effects` on the main thread.
- On Windows, original AstroBox's `Effect::Mica` maps directly to `window_vibrancy::apply_mica(window, None)`.
- No additional `DwmExtendFrameIntoClientArea` call or hidden acrylic layer exists in the Tauri implementation.
- Luna Studio's post-WebView Mica application is therefore equivalent in native timing; the remaining mismatch was frontend composition.

Frontend correction:
- Native-Mica `.app` now receives literal `transparent` through `--bg`.
- Added a separate `--workspace` material variable so contrast tint is bounded to the content workspace instead of covering the whole native window.
- Inset the native-Mica workspace by 8px and gave it one restrained 8px radius, matching original AstroBox's visible material rail/content separation.
- Dark mode keeps a 55% tint only inside the workspace; the sidebar is lighter at 58%, and the top bar is a 4% highlight over the bounded workspace.
- Unsupported systems, reduced-transparency mode, and High Contrast mode keep opaque workspace fallbacks.
- Added a narrow-window workspace margin/height correction for viewports at or below 680px.

Status: root-compositing correction complete; Mica-on/off screenshot comparison and build validation pending.

## Operation 11 - Proved that Mica was limited to the title bar and extended it into the client area

First controlled comparison:
- Used one unchanged 1196 x 819 window and captured it with DWM backdrop type `2`.
- Changed only DWM attribute 38 to type `1`, waited for composition, and captured the same window again.
- Restored type `2` before closing.
- Comparison sampled 241,164 pixels: only 8,198 reached an RGB delta of at least 9.
- Changed-pixel share was 3.40%; mean per-channel RGB delta was only 1.143.
- Visual inspection showed that nearly all meaningful difference was in the native title bar, not the WebView client area.
- The user's statement that Mica was not visibly present in the application body was therefore confirmed.

Validation-script note:
- The first C# image-comparison compile stopped before launching the app because PowerShell did not automatically reference `System.Drawing`.
- It made no DWM change and produced no screenshot.
- The script was rerun with an explicit `System.Drawing` reference and completed successfully.

Native correction:
- Added `DwmExtendFrameIntoClientArea` to the Windows Mica path.
- Uses margins `{-1, -1, -1, -1}` to extend the DWM frame across the complete client area.
- This follows Microsoft's documented full-window “sheet of glass” Win32 path.
- Luna Studio now assigns `native-mica` only when both client-frame extension and the current/fallback Mica attribute succeed.
- The call is still repeated after WebView2 creation and on Windows theme changes.

Status: client-area extension implemented; incremental rebuild and a second Mica-on/off comparison pending.

## Operation 12 - Found and fixed the white WebView host layer

Dependency comparison:
- Compared Tao 0.34.0, used by the original AstroBox stack, with the current Tao 0.35.3 Windows implementation.
- Both versions use the same `DwmEnableBlurBehindWindow` transparent-window block, so a Tao version regression was ruled out.
- Added read-only source snapshots for Tao 0.34.0, Tao 0.35.3, and Wry 0.55.1 under `reverse_apk/references`.
- Wry 0.55.1 correctly sets the WebView2 default background color to alpha zero when `.with_transparent(true)` is used.

Measured root cause:
- A pixel inside a completely transparent HTML root rail was exactly RGB `255,255,255`.
- Switching DWM backdrop type between Mica and none changed only the title bar while that white layer remained.
- Painting the native parent client red at runtime made the red color appear through the HTML root and all translucent CSS surfaces.
- This proved that WebView2 and the HTML root were genuinely transparent; the remaining blocker was the top-level Win32 client buffer, which was initialized white.
- `DwmExtendFrameIntoClientArea` by itself did not clear that existing white buffer.

Native fix:
- The Tao top-level window no longer uses its legacy transparent-window mode; only WebView2 remains transparent.
- Added `prepare_mica_surface`, called before WebView2 is created.
- It extends the DWM frame through the complete client area with margins `{-1,-1,-1,-1}`.
- It assigns the stock black brush to the native window class and invalidates the client before WebView2 creation.
- Black is the required glass-key surface for the extended DWM frame; the transparent WebView then reveals the actual system backdrop instead of the old white client buffer.
- Mica is still applied before and after WebView2 creation and again after theme changes.
- The HTML receives `native-mica` only when both native surface preparation and the Mica attribute succeed.

Result:
- A fresh process returned DWM attribute 38 value `2`.
- The formerly white root rail changed to a dark wallpaper-tinted material.
- The active window visibly picked up the purple desktop wallpaper color through the root, sidebar, workspace, and translucent controls.

Status: real client-area Mica established; frontend opacity refinement pending.

## Operation 13 - Increased frontend transparency after user review

User review:
- The user confirmed that Mica was now present and reported that the frontend elements were still not transparent enough.

Dark native-Mica palette:
- Main workspace changed from a 55% black cover to a 4% white material layer, matching the original AstroBox dark content approach.
- Sidebar changed from a 58% black cover to a 22% dark navigation tint.
- Standard surfaces now use 6.5% white, strong controls use 10%, soft surfaces use 7.5%, and fixed chrome uses 2.5%.
- Live preview, camera console, recording controls, and full media preview remain intentionally dark for accurate image display and stable control contrast.

Light native-Mica palette:
- Workspace reduced to 18% white.
- Standard surfaces reduced to 48%, strong surfaces to 68%, soft surfaces to 38%, chrome to 14%, and sidebar to 50% dark tint.
- Opaque unsupported-system, reduced-transparency, and High Contrast fallbacks were not weakened.

Controlled proof:
- Captured the same active 1100 x 740 window with DWM backdrop type `2`, changed only attribute 38 to type `1`, captured again, then restored type `2`.
- 513,055 of 814,000 pixels changed beyond the comparison threshold.
- Changed-pixel share: 63.03%.
- Mean per-channel RGB delta: 18.949.
- This replaces the earlier title-bar-only result of 3.40% changed pixels and proves that the client UI now composites over Mica.
- The 780 x 600 minimum-window capture retained collapsed navigation, readable controls, vertical scrolling, and no horizontal overflow.

Validation:
- Inline JavaScript syntax check passed.
- Static checks passed for transparent HTML/body, transparent native `.app`, one transparent WebView2, 4% dark workspace layer, and native black-glass preparation.
- `cargo test --bin html_app`: 10 passed, 0 failed.
- Release build completed with only the pre-existing unused OSC/dead-code warnings.

Status: implementation and visual validation complete; publication and cleanup pending.

## Operation 14 - Published and cleaned the real client-area Mica build

Publication:
- Rebuilt the release after UTF-8/CRLF normalization so the executable exactly matches the final source state.
- Replaced `F:/Insta360onWin/LunaStudio.exe` with the validated build.
- File size: 8,806,400 bytes.
- SHA-256: `FD3C21CF56C83F03BD0F2CB960169A8C02DF09CDC5F89F1B5BFE58C4B413BFD2`.

Published smoke test:
- The final executable created the `Luna 控制台` native window successfully.
- `DwmGetWindowAttribute(hwnd, 38)` returned HRESULT `0`, value `2` (`DWMSBT_MAINWINDOW`).
- `DwmGetWindowAttribute(hwnd, 20)` returned HRESULT `0`, value `1` (immersive dark mode).
- The process closed cleanly after the probe.

Reference repair:
- The first partial Tao 0.34.0 checkout was incomplete because its original network checkout missed blob objects.
- Removed that incomplete reference only after verifying its absolute F-drive path.
- Re-cloned exact tag `tao-v0.34.0`; the clean reference is commit `5ac00b57ad3f5c5c7135dde626cb90bc1ad469dc`.

Cleanup:
- Removed `LunaStudio-root-audit.exe` and its WebView2 profile.
- Removed `target_mica_root`.
- Removed the temporary `reverse_apk/validation/mica_root` screenshots and comparison files after recording their measurements here.
- Confirmed `LunaStudio.exe` is the only executable in the project root.
- No Luna Studio, audit, Cargo, or Rust compiler process remains.
- Retained only the exact source references used to explain the native behavior.

Encoding:
- Final Rust, HTML, and this handoff use UTF-8 without BOM and CRLF.

Status: complete. The published app now has a transparent HTML/WebView root over a real full-client native Mica surface, with user-reviewed higher-transparency frontend layers.

## Operation 15 - Preserved full-client Mica on the dedicated capture page

UI work:
- Added a separate capture page with live preview, compact capture-mode selector, contextual shutter, recording timer, and gimbal controls.
- Kept the HTML/WebView root transparent and retained the previously user-approved foreground opacity.
- Kept the live image surface opaque so camera pixels are not tinted by the desktop material.
- Verified 1180 x 780 and 760 x 560 layouts without horizontal overflow; the gimbal section remains reachable by vertical scrolling at minimum size.

Published verification:
- Final executable: `F:/Insta360onWin/LunaStudio.exe`.
- Size: 8,844,288 bytes.
- SHA-256: `1F74E00B7F8F33652898FBFFE2B22C8BF883273456D0F8DC7AA08F2AD59541E5`.
- Published smoke test returned backdrop HRESULT `0`, type `2`, and immersive-dark HRESULT `0`, value `1`.
- Full capture-control implementation and protocol notes are in `reverse_apk/CAPTURE_CONTROL_CONTINUE.md`.

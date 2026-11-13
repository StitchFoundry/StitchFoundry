# Progress

## Nightly log

- 2026-11-01 nightly sweep: no reports to triage; main CI green through tests (later steps stopped on a full disk); cargo audit has no vulnerabilities. Filed #31 and #32 (bug-report scrubber).
- 2026-11-01 fix for #31: names with letters like `İ` are now hidden by `help.bugReport.build`.
- 2026-10-31, issue 22 (part A, app side): `help.bugReport.build` / `.queue` / `.pending` in `crates/engine/src/cmd/bugreport.rs`. Scrubs emails, phones and masked names, caps sizes, marks reporter text untrusted, queues offline reports as files.
- Still to do for issue 22: the report form UI, the StitchHub endpoint (token, rate limiting, spam filter, reply channel) and the labels (needs-triage, triaged, cant-reproduce, needs-info, reproduced, fix-proposed, security). These live outside this repo or need a UI task.
- 2026-11-02 nightly sweep: no reported bugs to triage; no vulnerable dependencies (6 unmaintained notices); filed #39 (XXX reader 32-bit overflow) and #40 (hand-rolled date math).
- Fix #32: the bug-report scrub keeps ISO dates, numbers followed by a unit and space-grouped thousands instead of masking them as [phone].
- 2026-11-04, issue 52: AGENTS.md rewritten for StitchFoundry (upstream credit kept, issue/PR workflow added); CLAUDE.md is now `@AGENTS.md`.
- 2026-11-05 nightly sweep: no reports to triage; ran `cargo xtask ci` and cargo audit on main; one cloud-service finding kept private for the owner.
- Fix #47: embroidery file names (team names, queue, jobs, orders, patches, tiles) never end up as a Windows device name: `Con` exports as `Con-.dst`.
- Issue 61: customer-facing text names StitchFoundry instead of SiteSentrix (About, consent checkbox, README, website, installers, docs). Legal lines (LICENSE, NOTICE, copyright, ASSETS) untouched, waiting for Bruce. Consent-box translations dropped (English shows until retranslated).
- 2026-11-07 nightly sweep: no reports to triage; filed #74 (plain fill has no stitch cap). `cargo xtask ci` run on main; cargo audit attempted.
- Fix #66: decoration stitches (cord, sequins, beads, boring) stop at `decor::MAX_STITCHES` (200,000) per object, and a cord segment with a non-finite length is skipped.
- Issue 65: org move to StitchFoundry: links, workflow refs, update check, AppImage/packaging URLs and the fonts repo owner now say `StitchFoundry`; the Pages URL is `stitchfoundry.github.io`. App ids `com.stitchwright.*` and CHANGELOG history unchanged.
- 2026-11-08 nightly sweep: no reports to triage; `cargo xtask ci` on main green through fmt, clippy, tests and doctests (later steps stopped to save disk); cargo audit: no vulnerabilities (6 unmaintained, 1 yanked = #48). Filed #79 (run/zigzag/satin have no stitch cap).
- Fix #74: a fill piece stops at `fill::MAX_FILL_STITCHES` (1,000,000), and evenly or changing spaced fills have at most 200,000 rows (spread wider on a taller shape).
- 2026-11-09 nightly sweep: no reports to triage; cargo audit finds no vulnerabilities (7 unmaintained/yanked notices). Filed #84 (satin split length has no lower bound).
- Fix #79: run, zigzag, blanket, backstitch and stem lines stop at `run::MAX_RUN_STITCHES` (1,000,000 points per object, all passes), and satin sampling at `MAX_SATIN_PAIRS`. `polyline::points_at` walks a polyline once instead of once per point, so a long column is no longer quadratic.
- 2026-11-10 nightly sweep: no reports to triage; cargo audit finds no vulnerabilities (7 unmaintained/yanked notices); no new findings filed (the radial fill's split length, same bug as #84, is fixed with it).
- Fix #84: auto split and tatami pieces are at least `MIN_SPLIT_LENGTH` (0.5 mm) in satin columns, satin fills and radial fills, and one stitch is cut into at most 10,000 pieces.
- Issue 83 (part): `embroidery.newDesign` with no hoop named takes the last hoop picked by name (pref `last_hoop`, kept across restarts; ignored if My Machine lacks it; samples and custom sizes don't change it). Still to do: Ctrl/Cmd+N opening Start a design, on-screen out-of-field warning at creation, screenshots.
- 2026-11-13 nightly sweep: no reports to triage; no new findings filed (reviewed the Learn list loader, HUS decompressor, native unpack limits and the control server).
- Fix #40: `sf_formats::Timestamp::from_unix` reads the date from chrono instead of its own calendar math; past 9999 it gives 9999-12-31 23:59:59.
- Issue 94: Help → Report a Bug… / Suggest a Feature… open the hub forms `/report/stitchfoundry/bug` and `/feature` with `?v=<version>&system=`; no feature/bug `mailto:` links remain (test `no_report_link_is_a_mailto`). Only `support@stitchfoundry.app` text in cloud service emails/errors (apps/stitchfoundry-cloud/src/api.rs) stays.

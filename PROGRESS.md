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
- 2026-11-08 nightly sweep: no reports to triage; `cargo xtask ci` on main green through fmt, clippy, tests and doctests (later steps stopped to save disk); cargo audit: no vulnerabilities (6 unmaintained, 1 yanked = #48). Filed #79 (run/zigzag/satin have no stitch cap).
- Fix #74: a fill piece stops at `fill::MAX_FILL_STITCHES` (1,000,000), and evenly or changing spaced fills have at most 200,000 rows (spread wider on a taller shape).
- 2026-11-09 nightly sweep: no reports to triage; cargo audit finds no vulnerabilities (7 unmaintained/yanked notices). Filed #84 (satin split length has no lower bound).
- Fix #79: run, zigzag, blanket, backstitch and stem lines stop at `run::MAX_RUN_STITCHES` (1,000,000 points per object, all passes), and satin sampling at `MAX_SATIN_PAIRS`. `polyline::points_at` walks a polyline once instead of once per point, so a long column is no longer quadratic.

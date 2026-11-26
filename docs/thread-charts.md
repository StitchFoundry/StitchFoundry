# Built-in thread charts

Status: infrastructure only, no maker chart shipped yet (issue 212). Revised 2026-11-26.

> Thread colour data is approximate and for matching only. Not affiliated with or endorsed by the
> thread makers. Screen colours are not thread colours: test-sew on scrap.

Charts are plain labels (code, name, approximate colour); no maker logos or artwork. Each maker's chart
goes in its own file (`crates/sf-core/src/thread_makers/<maker>.rs`), registered in
`thread_makers::MAKERS`, so it can be removed alone. `thread_makers::ENABLED` is the one list of
shown charts: remove a name to hide the chart in the pickers, matching and export.

Use only data the maker publishes for reuse or that is plain fact. Not from files, cards or
catalogues that forbid redistribution, paywalled or logged-in areas, maker software or its data
files, or scraping a site forbids.

| Maker | Chart | Source (URL or document) | Date read | Entries | Status |
|---|---|---|---|---|---|
| Isacord | | isacordthread.com terms forbid copying site data | 2026-11-22 | 0 | skipped: no clean source |
| Madeira | | not checked | | 0 | open |
| Robison-Anton | | not checked | | 0 | open |
| Sulky | | not checked | | 0 | open |

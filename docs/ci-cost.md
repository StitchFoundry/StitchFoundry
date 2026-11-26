# CI cost

> **Last updated:** 2026-11-25 · **Change:** first measurement (issue #267, part 1) and the docs-only / draft skip design.

Goal (issue #267): cut GitHub Actions spend by half or more without losing any check that protects `main`.
Bill for 2026-11-01..25 (billing report, read-only): about $55 net. 51,808 Linux min (~$33), 873 macOS min
(~$18, 10x rate), 3,546 Windows min (~$2), rest storage.

## Before: runs in the last 30 days (2026-10-26 to 2026-11-25)

Source: `gh run list --limit 1000 --created ">=2026-10-26" --json name,event,startedAt,updatedAt`. Minutes are
wall-clock per run (`updatedAt - startedAt`, summed over jobs' parallelism is not included), so they are
**estimates**; the billing report is authoritative. The 1000-run cap was reached, so the oldest days may be
missing. Dependabot "dynamic" runs and `Claude fixes bugs` issue-label runs are under 10 min total and left out.

| Workflow | Trigger | Runs | ~Minutes | Runner | Cost class |
|---|---|---:|---:|---|---|
| CI (`ci.yml`) | pull_request | 93 | 4,737 | Linux | required check |
| FreeBSD | pull_request | 72 | 4,312 | Linux (VM) | heavy, rarely fails |
| CI | push to main | 57 | 1,977 | Linux | keep |
| FreeBSD | push to main | 43 | 1,799 | Linux (VM) | keep on main |
| Affinity 3 corpus | pull_request | 69 | 1,427 | Linux | heavy, rarely fails |
| SonarCloud | pull_request | 93 | 1,074 | Linux | cheap-ish |
| Web app (`pages.yml`) | push to main | 57 | 953 | Linux | keep |
| Claude fixes bugs | issue_comment | 177 | 921 | Linux | agent runs |
| Desktop printing (`print.yml`) | pull_request | 18 | 810 | **Windows** | path filtered |
| Affinity 3 corpus | push to main | 43 | 396 | Linux | keep on main |
| Windows ARM64 | nightly schedule | 9 | 309 | Windows | already nightly |
| SonarCloud | push to main | 57 | 141 | Linux | keep |
| Release | push | 1 | 88 | mixed | never touch |
| Everything else (Nix, Packaging lint, dependabot) | | | ~60 | Linux | negligible |

macOS and most Windows minutes come from `builds.yml` / `windows-build.yml` (push to the `builds` and
`windows-build` branches, `workflow_dispatch`) and `release.yml`; none of them runs on PRs. Concurrency
(`cancel-in-progress`) is already set in `ci`, `affinity-corpus`, `freebsd`, `nix`, `packaging-lint`, `builds`,
`windows-*`, `sonarcloud`; `claude-fix.yml` has none (part 3).

Reading the table: on PRs, `cargo xtask ci` is ~4.7k min, FreeBSD ~4.3k, Affinity ~1.4k, SonarCloud ~1.1k,
Desktop printing ~0.8k (Windows, 10x less than macOS but 2x Linux). FreeBSD plus Affinity are as costly as CI itself.
Many of those PR runs are docs-only or draft pushes (agents open draft PRs early and push often).

## Part 1 (this PR): docs-only and draft skip

Rules, to apply in every PR-triggered heavy workflow (`ci.yml`, `freebsd.yml`, `affinity-corpus.yml`,
`sonarcloud.yml`, `print.yml`):

1. **Docs-only**: a PR whose changed files are all `*.md`, `docs/**`, `progress/**`, `ROADMAP.md`, `PROGRESS.md`
   runs only a cheap job.
2. **Draft**: heavy jobs do not run while the PR is a draft; add `ready_for_review` to the `pull_request` types
   so marking it ready starts them.
3. **Required check stays reportable.** Do **not** use trigger-level `paths-ignore` on `ci.yml`: a required check
   that never starts stays "Expected" and blocks the merge. Instead the `ci` job always starts, and its heavy
   steps are skipped when nothing code-like changed.

Proposed `ci.yml` change (workflow files are not editable by the agent app, so Bruce applies it):

```yaml
on:
  push:
    branches: [main]
  pull_request:
    types: [opened, synchronize, reopened, ready_for_review]
  workflow_dispatch:

jobs:
  ci:
    # Drafts skip: a skipped job reports as success for branch protection.
    if: github.event_name != 'pull_request' || github.event.pull_request.draft == false
    runs-on: ubuntu-latest
    timeout-minutes: 150
    steps:
      - uses: actions/checkout@v4
        with: { fetch-depth: 0 }
      - name: Detect docs-only change
        id: scope
        run: |
          code=true
          if [ "${{ github.event_name }}" = pull_request ]; then
            base="${{ github.event.pull_request.base.sha }}"
            if ! git diff --name-only "$base"...HEAD | grep -qvE '(\.md$|^docs/|^progress/)'; then
              code=false
            fi
          fi
          echo "code=$code" >> "$GITHUB_OUTPUT"
      # every existing step after this one gets: if: steps.scope.outputs.code == 'true'
```

For `freebsd.yml`, `affinity-corpus.yml` and `print.yml` (not required checks, already path-filtered to code),
only the draft rule is needed: `if: github.event.pull_request.draft == false` on each job and the
`ready_for_review` type. `sonarcloud.yml` gets the same draft rule.

Docs-only PR after the change: `ci` starts, runs checkout plus one `git diff` (about 20 s) and passes; the other
workflows are path-filtered or skip.

## Estimated after (parts 1 to 3)

Assumes about half of PR pushes are docs-only or on drafts (the 93 CI PR runs include many agent draft pushes);
**estimate, not measured**.

| Change | Saves ~min / 30 days | Part |
|---|---:|---|
| Docs-only and draft skip on `ci` | ~2,300 | 1 |
| Draft skip on SonarCloud, Desktop printing | ~900 | 1 |
| FreeBSD and Affinity off PR pushes (main, nightly, label `full-ci`) | ~5,700 | 2 |
| `concurrency` on `claude-fix.yml` (cancel stale agent runs) | ~150 | 3 |
| Artifact and cache retention (storage) | ~$1 to $2 | 3 |

Estimated PR minutes: about 12,400 now (CI 4,737 + FreeBSD 4,312 + Affinity 1,427 + SonarCloud 1,074 + printing 810), about 3,500 after (-70%); main, nightly and release runs unchanged.

## Revision history

- 2026-11-25: first measurement and part 1 design (issue #267).

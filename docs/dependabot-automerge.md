# Dependabot auto-merge

Status: pilot. Workflow: `.github/workflows/dependabot-automerge.yml`. Revision: 2026-11-24, first version.

On every pull request opened by `dependabot[bot]` the workflow reads the update metadata
(`dependabot/fetch-metadata`, pinned by commit SHA) and turns on squash auto-merge
(`gh pr merge --auto --squash`) only when **every** dependency in the PR is:

- a `version-update:semver-patch` or `version-update:semver-minor` update,
- not a 0.x minor bump (0.x minors count as breaking),
- not in the `github_actions` ecosystem (workflow-file bumps stay manual).

A grouped update is checked dependency by dependency: one major, 0.x minor or
`github_actions` entry makes the whole PR manual. Otherwise the workflow does nothing
except post one comment saying why (marked, so it is never repeated).

The workflow merges nothing itself. It only enables auto-merge, so GitHub still waits for all
required status checks and branch rules before merging.

Agent (`claude/*`) and human PRs are never touched: the job only runs for `dependabot[bot]`.
Permissions are `contents: write` and `pull-requests: write` for that one job; no secrets
beyond the built-in `GITHUB_TOKEN`; no `pull_request_target`.

## Turning it off

Delete the workflow file, or disable `dependabot-automerge` under Actions. Pending auto-merges
can be cancelled per PR with `gh pr merge --disable-auto <PR>`.

## Rollout

Piloted in patternfoundry first; other repos follow once it has handled a real Dependabot PR.

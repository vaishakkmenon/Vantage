# NAS transfer note (2026-09-16)

This branch (`nas-transfer/wip-2026-09-16`) is a temporary snapshot branch created
before moving this repo from a local Windows machine to a NAS.

- Base branch at time of snapshot: `feature/backend_agentic_updates`
- Purpose: preserve unpushed commits and uncommitted working-tree changes so nothing
  is lost during the transfer. The WIP commit on this branch is a raw dump of the
  working tree, not reviewed, not necessarily building or passing tests.

## For Claude / future me

- Do not treat the WIP commit as finished work. Treat it as a stash.
- To resume: checkout `feature/backend_agentic_updates`, then cherry-pick or `git merge --squash` from this
  branch, review the diff, and split into real commits.
- Once merged or discarded, delete this branch and this file.

# MVP 0 handoff

The import is published on DEV. Foundation work continues on `feature/mvp0-foundation` for Issue #1. Use `git log -1 origin/feature/mvp0-foundation` for the last published checkpoint. DEV combines both original histories with a two-parent merge. The upstream baseline is v2.7.3 at `2ba32ea43246bd4db0265eb3120d3c72e39f5b22`, also observed as upstream main.

The imported project and original BeyGram roadmap are present. The source audit, release workflow isolation, documentation and build verification are pending. No build has run at this checkpoint. Reproduce the ancestry checks with `git merge-base --is-ancestor 2ba32ea43246bd4db0265eb3120d3c72e39f5b22 origin/DEV` and the same command with `origin/main` as the first revision. Next read the workspace manifests, Makefile and upstream workflows.

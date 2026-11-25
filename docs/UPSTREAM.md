# Upstream baseline

[Impactor v2.7.3](https://github.com/claration/Impactor/releases/tag/v2.7.3) was published on November 17, 2026. Its commit, `2ba32ea43246bd4db0265eb3120d3c72e39f5b22`, was also the tip of upstream's default development branch `main` on November 25. The preceding stable releases were v2.7.2 and v2.7.1. Using the current stable commit ties the audit to a published baseline without taking unreleased changes.

DEV starts with merge commit `bc0320389fe6d540b28a291c757c91c39d3ecffe`. Its first parent is the BeyGram planning commit `7d44044be60ddc5fa758709d37d81ced8dc7499c`. Its second parent is the original upstream commit. The full ancestry of both parents remains reachable with original commit SHAs and authors. Upstream topic branches are fetched locally for inspection. Upstream tags are intentionally kept local because the imported release workflow listens for version tags.

Configure each clone with the upstream remote and use a review branch for synchronization:

```sh
git remote add upstream https://github.com/claration/Impactor.git
git fetch --no-tags upstream
git fetch --no-tags upstream tag v2.7.3
git merge-base --is-ancestor v2.7.3 origin/DEV
git merge-base --is-ancestor 7d44044be60ddc5fa758709d37d81ced8dc7499c origin/DEV
git switch -c sync/impactor-next origin/DEV
```

Inspect the next candidate with `git log <current-sha>..<candidate-sha>` and `git diff <current-sha> <candidate-sha>`, including workflows, binaries, licenses and lockfile. Fetch its tag explicitly if using a release. Merge the chosen SHA with `git merge --no-ff <candidate-sha>`, run the build checks and record findings here. Push the sync branch and open a PR to DEV. Use a merge commit to keep the new upstream ancestry reachable. Keep notices and resolve repository-specific workflow changes during review. The original upstream README lives at [.github/README.md](../.github/README.md).

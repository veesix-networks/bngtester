# Code Review: CI Pipeline to Publish Subscriber Images (Codex)

I reviewed `.github/workflows/publish-images.yml` against the finalized spec in `context/specs/2-ci-publish-dockerhub/IMPLEMENTATION_SPEC.md`. The core architecture matches what the spec asked for: three jobs (`discover -> build -> push`), dynamic discovery from `images/*/Dockerfile`, shared `images/` build context, per-image naming, PR build-only behavior, OCI metadata, GHA caching, explicit `linux/amd64`, and SPDX headers are all present.

## Findings

### MEDIUM: non-semver `v*` tags still match the workflow trigger, so the implementation is broader than the finalized trigger rules

- The finalized spec says release publishing is for semver tags and explicitly treats a tag like `vnext` as a non-semver case that should not enter the normal publish path (`context/specs/2-ci-publish-dockerhub/IMPLEMENTATION_SPEC.md:41-42`, `context/specs/2-ci-publish-dockerhub/IMPLEMENTATION_SPEC.md:46-52`, `context/specs/2-ci-publish-dockerhub/IMPLEMENTATION_SPEC.md:174-175`).
- The workflow still triggers on any tag matching `v*` (`.github/workflows/publish-images.yml:8-10`), and the `push` job runs on every non-PR event (`.github/workflows/publish-images.yml:62-64`).
- `docker/metadata-action` does narrow generated tags with `type=semver` (`.github/workflows/publish-images.yml:47-49`, `.github/workflows/publish-images.yml:84-86`), but the semver restriction exists only in metadata generation, not in the workflow trigger itself. So non-semver tags still execute the workflow even though the spec documents semver tag pushes as the trigger rule.
- Recommendation: tighten `on.push.tags` to a semver-compatible pattern, or add an explicit guard that skips the publish path when the ref is not a valid semver tag.

### MEDIUM: the workflow does not fully deliver the spec's "never partially-updated" failure guarantee

- The finalized spec says the three-job design "ensures Docker Hub is never left in a partially-updated state from a failed workflow run" (`context/specs/2-ci-publish-dockerhub/IMPLEMENTATION_SPEC.md:112-120`).
- The implementation does satisfy the narrower build-stage guarantee: `push` depends on `build`, and the build job uses `push: false`, so a Dockerfile build failure cannot publish anything (`.github/workflows/publish-images.yml:31-61`, `.github/workflows/publish-images.yml:62-97`).
- But the `push` phase itself is still a matrix. If one image push succeeds and another fails because of Docker Hub or network problems, the workflow ends failed after some tags have already been published. `fail-fast: true` cancels remaining legs; it does not roll back successful pushes (`.github/workflows/publish-images.yml:66-69`, `.github/workflows/publish-images.yml:88-97`).
- Recommendation: either relax the spec wording to say the build gate prevents build-stage partial publication while push-stage partial publication is an accepted rerun scenario, or change the implementation if atomic publication is truly required.

## Notes

- I did not find a compliance gap in dynamic discovery. The `find ... Dockerfile` step and matrix fan-out match the finalized spec (`.github/workflows/publish-images.yml:15-30`, `.github/workflows/publish-images.yml:31-37`, `.github/workflows/publish-images.yml:68-69`).
- Image naming, shared `images/` build context, `pull_request` build-only behavior, `platforms: linux/amd64`, GHA caching, OCI labels, Docker Hub login, and SPDX headers all match the finalized spec.
- I did not execute the workflow in GitHub Actions or inspect live Docker Hub state in this review, so this is a static compliance review of the checked-in workflow file.

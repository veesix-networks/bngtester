# Spec Critique: CI Pipeline to Publish Subscriber Images to Docker Hub (Codex)

The shared build-context requirement is captured correctly, and the spec stays within the intended single-workflow scope. The main gaps are around release semantics and failure handling: the current draft can publish the wrong `latest`, can leave Docker Hub partially updated when one matrix leg fails, and only "builds all Dockerfiles under `images/`" as long as a hardcoded list is kept manually in sync.

## Findings

### HIGH: the tag strategy in the spec does not match issue #2 and can move `latest` away from `main`

- Issue #2 says pushes to `main` publish `latest`, and semver tag pushes publish versioned releases. The spec's trigger table and tag strategy instead define tag pushes as `v*` and add `latest` on tag events too (`context/specs/2-ci-publish-dockerhub/IMPLEMENTATION_SPEC.md:38`, `context/specs/2-ci-publish-dockerhub/IMPLEMENTATION_SPEC.md:46`).
- `v*` is broader than semver. Tags like `vnext` or `vfoo` still match the workflow trigger unless the implementation adds a second validation gate.
- Publishing `latest` from a tag event creates a real architectural ambiguity: a release tag pushed from an older commit or a ref that is not the current `main` tip can overwrite the `latest` image, even though the issue defines `latest` as the `main` branch tag.
- The spec should make the rule explicit: `latest` is published only on pushes to `main`, and tag events publish only version tags parsed from valid semver refs. If release tags are also supposed to move `latest`, that needs an explicit issue amendment because it is broader than the current acceptance criteria.

### HIGH: `fail-fast` does not address the partial-publish failure mode

- The spec equates `fail-fast: true` with the acceptance criterion "Build fails if any Dockerfile fails to build" (`context/specs/2-ci-publish-dockerhub/IMPLEMENTATION_SPEC.md:67`).
- That only describes GitHub Actions job status. It does not protect Docker Hub from partial publication. In a matrix, one image can finish `docker/build-push-action` and publish `latest` or `0.1.0` before another leg fails on build, auth, rate limit, or network push.
- The resulting state is a failed workflow paired with a mixed registry state: some distros updated, others still point at the previous build. That is especially risky for shared release tags because users can pull a partially published release that the workflow itself reports as failed.
- The spec needs an explicit failure contract here.
- One option is to split "build all images" from "publish images" so publishing starts only after every image build succeeds.
- Another is to accept partial publication as an explicit tradeoff and document the cleanup/rerun procedure.
- Another is to add retry/concurrency rules for transient Docker Hub failures so a single `429` or timeout does not leave the release in an undefined state.

### MEDIUM: hardcoding `[alpine, debian, ubuntu]` does not fully satisfy "builds all Dockerfiles under `images/`" and will drift

- The issue says the workflow builds each image in `images/`, and the spec overview says this pipeline should unblock future subscriber images without manual build/push (`context/specs/2-ci-publish-dockerhub/IMPLEMENTATION_SPEC.md:5`).
- The design and implementation plan hardcode the matrix to the current three directories (`context/specs/2-ci-publish-dockerhub/IMPLEMENTATION_SPEC.md:23`, `context/specs/2-ci-publish-dockerhub/IMPLEMENTATION_SPEC.md:27`, `context/specs/2-ci-publish-dockerhub/IMPLEMENTATION_SPEC.md:101`).
- That is enough for today's repo state, but it is not enough for the stronger contract the spec claims. As soon as a fourth image is added and the workflow file is not edited in the same PR, the pipeline stops matching "all Dockerfiles under `images/`."
- The spec should either define a discovery step that derives the matrix from `images/*/Dockerfile`, or explicitly narrow the goal to the current three images and remove the stronger "all Dockerfiles" / "future images without manual build/push" language.

### LOW: the amd64-only boundary is stated, but not turned into an implementation requirement

- The issue and spec both say multi-arch builds are out of scope and this workflow is amd64-only (`context/specs/2-ci-publish-dockerhub/IMPLEMENTATION_SPEC.md:129`).
- The implementation plan does not say how the workflow guarantees that. With Buildx, the effective architecture is currently just whatever runner architecture GitHub provides, which is an implicit assumption rather than an encoded boundary.
- The spec should say either "set `platforms: linux/amd64`" or explicitly rely on GitHub-hosted amd64 runners and avoid any QEMU or multi-platform setup. Otherwise the scope boundary exists in prose only.

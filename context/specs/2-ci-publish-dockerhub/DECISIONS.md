# Decisions: 2-ci-publish-dockerhub

## Accepted

### `latest` tag only from `main`, strict semver for tag events
- **Source:** CODEX
- **Severity:** HIGH
- **Resolution:** Tag strategy rewritten. `latest` is published only on pushes to `main`. Tag events use `type=semver` for strict semver parsing — non-semver tags like `vnext` produce no output and no push occurs.

### Split build and push to prevent partial publication
- **Source:** CODEX
- **Severity:** HIGH
- **Resolution:** Workflow redesigned into three jobs: discover → build (push: false) → push (needs: build). The push job only runs after all build legs succeed, preventing partial Docker Hub state. Re-running the workflow is the documented remediation for transient push failures.

### Dynamic image discovery instead of hardcoded matrix
- **Source:** CODEX
- **Severity:** MEDIUM
- **Resolution:** Added a discover job that finds `images/*/Dockerfile` directories dynamically (excluding `shared/`). The build and push jobs consume the output as a matrix. Adding a new image requires only its Dockerfile — no workflow edits needed.

### Explicitly set `platforms: linux/amd64`
- **Source:** CODEX
- **Severity:** LOW
- **Resolution:** Added `platforms: linux/amd64` to the build-push-action configuration. The amd64-only scope boundary is now encoded in the workflow rather than relying on implicit runner architecture.

### Add `pull_request` trigger for build-only CI verification
- **Source:** GEMINI
- **Severity:** MEDIUM
- **Resolution:** Added `pull_request` trigger targeting `main`. On PR events, the workflow runs discover and build jobs (push: false) to validate Dockerfile changes before merge. The push job is skipped.

### Set `metadata-action` image name per matrix iteration
- **Source:** GEMINI
- **Severity:** MEDIUM
- **Resolution:** Spec now explicitly states `images: veesix/bngtester-${{ matrix.image }}` in the metadata-action configuration, ensuring correct image names and OCI metadata per matrix leg.

### Add GitHub Actions build caching
- **Source:** GEMINI
- **Severity:** LOW
- **Resolution:** Added `cache-from: type=gha` and `cache-to: type=gha,mode=max` to the build-push-action configuration. Mitigates the cost of the two-phase build (build then push) by caching layers.

### Add OCI labels via metadata-action
- **Source:** GEMINI
- **Severity:** LOW
- **Resolution:** Spec now documents that `docker/metadata-action` automatically injects standard OCI labels from GitHub context. The `labels` output is passed to the build-push-action.

### `latest` tag race condition — moot
- **Source:** GEMINI
- **Severity:** LOW
- **Resolution:** Finding is moot after accepting Codex C1 — `latest` is now exclusively published from `main` pushes, eliminating the race between `main` and tag events.

## Rejected

None.

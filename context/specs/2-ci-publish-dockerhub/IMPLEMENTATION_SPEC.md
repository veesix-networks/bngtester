# Implementation Spec: CI Pipeline to Publish Subscriber Images to Docker Hub

## Overview

GitHub Actions workflow that automatically builds all subscriber container images under `images/` and publishes them to Docker Hub. Triggers on pushes to `main` (tagged `latest`), on semver tags (tagged with the version), and on pull requests (build-only, no push). This is the first CI pipeline in the repo and unblocks adding future subscriber images without manual build/push.

## Source Issue

[#2 — CI pipeline to build and publish subscriber images to Docker Hub](https://github.com/veesix-networks/bngtester/issues/2)

## Current State

- Three subscriber images exist: `images/alpine/`, `images/debian/`, `images/ubuntu/`
- All Dockerfiles use a shared build context of `images/` (e.g., `docker build -f images/alpine/Dockerfile images/`) to access `images/shared/entrypoint.sh`
- No CI pipelines exist — `.github/workflows/` directory does not exist
- Docker Hub organization is `veesix`
- Naming convention from the issue: `veesix/bngtester-<image-name>` (e.g., `veesix/bngtester-alpine`)

## Design

### Workflow Architecture

A single workflow file with three jobs: **discover**, **build**, and **push**. The discover job dynamically finds all image directories. The build job validates all Dockerfiles compile. The push job publishes only after all builds succeed, preventing partial publication.

```
Event ──> discover ──> build (matrix) ──> push (matrix)
                         │                   │
                         ├── alpine ✓         ├── push alpine
                         ├── debian ✓         ├── push debian
                         └── ubuntu ✓         └── push ubuntu
                         (all must pass)    (only if all builds passed)
```

On `pull_request` events, only discover and build run (push is skipped). This validates Dockerfile changes before merge.

### Trigger Rules

| Event | Condition | Image Tag | Pushes to Registry |
|-------|-----------|-----------|-------------------|
| Push to `main` | Any push (merge, direct push) | `latest` | Yes |
| Tag push | Tag matches semver pattern (e.g., `v0.1.0`) | Version from tag (e.g., `0.1.0`) | Yes |
| Pull request | Targets `main` | N/A | No (build-only) |

### Tag Strategy

Uses `docker/metadata-action` to derive tags with `type=semver` for strict semver parsing:

- **On push to `main`:** Tag as `latest` only
- **On semver tag:** Tag as the semver version (e.g., `v0.1.0` → `0.1.0`). Does **not** update `latest` — `latest` strictly follows `main`
- **On pull request:** No tags pushed (build-only validation)

Non-semver tags (e.g., `vnext`, `vfoo`) do not match `type=semver` and produce no tags, so no images are pushed.

### Dynamic Image Discovery

The workflow does not hardcode the image list. A discover job finds all directories under `images/` that contain a `Dockerfile`, excluding `shared/`:

```yaml
discover:
  steps:
    - id: find
      run: |
        images=$(find images -name Dockerfile -mindepth 2 -maxdepth 2 \
          | xargs -I{} dirname {} | xargs -I{} basename {} \
          | grep -v '^shared$' \
          | jq -Rcn '[inputs]')
        echo "images=$images" >> "$GITHUB_OUTPUT"
```

The build and push jobs consume this as `matrix.image: ${{ fromJson(needs.discover.outputs.images) }}`. Adding a new subscriber image only requires adding its `images/<distro>/Dockerfile` — the workflow picks it up automatically.

### Image Naming

Each image is published as `veesix/bngtester-<distro>`. The `docker/metadata-action` `images` input is set per matrix iteration:

```yaml
images: veesix/bngtester-${{ matrix.image }}
```

| Directory | Docker Hub Image |
|-----------|-----------------|
| `images/alpine/` | `veesix/bngtester-alpine` |
| `images/debian/` | `veesix/bngtester-debian` |
| `images/ubuntu/` | `veesix/bngtester-ubuntu` |

### Build Context

All Dockerfiles use `images/` as the build context (not the per-image subdirectory), because they `COPY shared/entrypoint.sh`. The workflow must replicate this:

```yaml
context: images/
file: images/${{ matrix.image }}/Dockerfile
```

### Platform

Builds target `linux/amd64` explicitly via `platforms: linux/amd64` in the build-push-action. This encodes the amd64-only scope boundary rather than relying on the implicit runner architecture.

### Build Caching

Uses GitHub Actions cache backend for Docker layer caching:

```yaml
cache-from: type=gha
cache-to: type=gha,mode=max
```

### OCI Labels

`docker/metadata-action` automatically injects standard OCI labels (`org.opencontainers.image.source`, `org.opencontainers.image.revision`, `org.opencontainers.image.created`, etc.) from the GitHub context. The `labels` output is passed to `docker/build-push-action`.

### Failure Behavior

The workflow uses a three-job pipeline to prevent partial publication:

1. **discover** — finds image directories
2. **build** — matrix job, builds all images with `push: false`. Uses `fail-fast: true`. If any image fails to build, the entire workflow fails here.
3. **push** — matrix job, `needs: [discover, build]`. Only runs if **all** build legs succeeded. Builds and pushes each image.

This ensures Docker Hub is never left in a partially-updated state from a failed workflow run. If a transient push failure occurs in the push job, re-running the workflow is the remediation.

## Configuration

### Repository Secrets (required)

| Secret | Purpose |
|--------|---------|
| `DOCKERHUB_USERNAME` | Docker Hub username for authentication |
| `DOCKERHUB_TOKEN` | Docker Hub access token (not password) |

These must be configured in the repository settings before the workflow can push images.

### No Environment Variables

The workflow does not introduce any new environment variables for local development. All configuration is within the workflow file itself.

## File Plan

| File | Action | Purpose |
|------|--------|---------|
| `.github/workflows/publish-images.yml` | Create | GitHub Actions workflow for building and publishing images |
| `context/specs/2-ci-publish-dockerhub/IMPLEMENTATION_SPEC.md` | Create | This spec |
| `context/specs/2-ci-publish-dockerhub/README.md` | Create | Status tracker |
| `context/specs/2-ci-publish-dockerhub/DECISIONS.md` | Create | Review decision log |

## Implementation Order

### Phase 1: Workflow File

Create `.github/workflows/publish-images.yml` with:

1. Trigger configuration (`push` to `main`, semver tags, `pull_request` to `main`)
2. Discover job — dynamic image directory discovery
3. Build job — matrix build with `push: false`, `platforms: linux/amd64`, GHA caching
4. Push job — matrix build+push, `needs: [discover, build]`, skipped on PRs
5. `docker/metadata-action` with `images: veesix/bngtester-${{ matrix.image }}`, `type=semver` tags, OCI labels
6. Docker Hub login using repository secrets
7. SPDX copyright header

This is a single logical unit — the workflow file is self-contained and independently testable.

### Verification

After merging to `main`, the workflow triggers automatically. Manual verification:

1. Check the Actions tab for a successful run
2. Verify images appear on Docker Hub under `veesix/bngtester-alpine`, `veesix/bngtester-debian`, `veesix/bngtester-ubuntu`
3. Pull and run an image to confirm it works: `docker pull veesix/bngtester-alpine:latest`

## Testing

- **PR validation:** Open a PR that touches a Dockerfile — the workflow should build but not push
- **Build validation:** Push to `main` — the workflow runs and publishes `latest` tags
- **Tag validation:** Create a tag `v0.1.0` and verify images are tagged `0.1.0` on Docker Hub (but `latest` is not updated)
- **Non-semver tag:** Push a tag like `vnext` and verify no images are published
- **Failure validation:** Introduce a deliberate Dockerfile error and verify the workflow fails at the build stage without publishing any images
- **New image discovery:** Add a new `images/<distro>/Dockerfile` and verify the workflow picks it up without editing the workflow file
- **Local build test:** Each Dockerfile can be built locally with `docker build -f images/<distro>/Dockerfile images/` to verify before CI

## Not In Scope

- Multi-arch builds (amd64 only, per issue — encoded via `platforms: linux/amd64`)
- Collector binary builds
- Image signing or attestation
- Automated testing of built images (run containers, verify connectivity)
- Path-based triggering (only build images whose Dockerfiles changed) — all images build on every trigger for simplicity

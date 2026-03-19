# Implementation Spec: CI Pipeline to Publish Subscriber Images to Docker Hub

## Overview

GitHub Actions workflow that automatically builds all subscriber container images under `images/` and publishes them to Docker Hub. Triggers on pushes to `main` (tagged `latest`) and on semver tags (tagged with the version). This is the first CI pipeline in the repo and unblocks adding future subscriber images without manual build/push.

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

A single workflow file handles all images using a matrix strategy. Each image in the matrix maps to a subdirectory under `images/` that contains a `Dockerfile`.

```
Push to main ──┐
               ├──> publish-images.yml ──> matrix [alpine, debian, ubuntu]
Semver tag ────┘                              │
                                              ├── build veesix/bngtester-alpine
                                              ├── build veesix/bngtester-debian
                                              └── build veesix/bngtester-ubuntu
```

### Trigger Rules

| Event | Condition | Image Tag |
|-------|-----------|-----------|
| Push to `main` | Any push (merge, direct push) | `latest` |
| Tag push | Tag matches `v*` (e.g., `v0.1.0`, `v1.0.0-rc.1`) | Version from tag (e.g., `0.1.0`) |

### Tag Strategy

Uses `docker/metadata-action` to derive tags:

- **On push to `main`:** Tag as `latest`
- **On semver tag `v*`:** Tag as the semver version (e.g., `v0.1.0` → `0.1.0`), plus `latest`

### Image Naming

Each image is published as `veesix/bngtester-<distro>`:

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

### Failure Behavior

The matrix strategy uses `fail-fast: true` (default). If any image fails to build, the entire workflow fails. This matches the acceptance criteria: "Build fails if any Dockerfile fails to build."

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

## Implementation Order

### Phase 1: Workflow File

Create `.github/workflows/publish-images.yml` with:

1. Trigger configuration (`push` to `main`, tag `v*`)
2. Matrix strategy listing all three images
3. Steps:
   - Checkout repository
   - Set up Docker Buildx
   - Log in to Docker Hub using repository secrets
   - Extract metadata (tags, labels) using `docker/metadata-action`
   - Build and push using `docker/build-push-action`
4. SPDX copyright header

This is a single logical unit — the workflow file is self-contained and independently testable.

### Verification

After merging to `main`, the workflow triggers automatically. Manual verification:

1. Check the Actions tab for a successful run
2. Verify images appear on Docker Hub under `veesix/bngtester-alpine`, `veesix/bngtester-debian`, `veesix/bngtester-ubuntu`
3. Pull and run an image to confirm it works: `docker pull veesix/bngtester-alpine:latest`

## Testing

- **Build validation:** Push to `main` or create a tag — the workflow runs automatically
- **Tag validation:** Create a tag `v0.1.0` and verify images are tagged `0.1.0` on Docker Hub
- **Failure validation:** Introduce a deliberate Dockerfile error and verify the workflow fails
- **Local build test:** Each Dockerfile can be built locally with `docker build -f images/<distro>/Dockerfile images/` to verify before CI

## Not In Scope

- Multi-arch builds (amd64 only, per issue)
- Collector binary builds
- Image signing or attestation
- Automated testing of built images (run containers, verify connectivity)
- Build caching optimization (can be added later if build times are a concern)
- Path-based triggering (only build images whose Dockerfiles changed) — all images build on every trigger for simplicity

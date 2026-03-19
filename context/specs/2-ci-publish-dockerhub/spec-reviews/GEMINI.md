# Spec Review: CI Pipeline to Publish Subscriber Images to Docker Hub (Gemini)

## Overview

The specification provides a solid foundation for the initial CI/CD pipeline. It correctly identifies the shared build context and uses a matrix strategy to manage the three existing subscriber images. The use of standard GitHub Actions like `docker/build-push-action` and `docker/metadata-action` aligns with industry best practices.

## Findings

### HIGH

No high-severity findings identified.

### MEDIUM

#### 1. Lack of `pull_request` Trigger for CI Verification
- **Source:** GEMINI
- **Severity:** MEDIUM
- **Finding:** The spec only triggers on `push` to `main` and `v*` tags. This means that errors in Dockerfiles or the workflow itself will only be caught *after* they are merged into the main branch or when a release is attempted.
- **Recommendation:** Add a `pull_request` trigger that runs the build (but does **not** push) to verify that changes are valid before they are merged.

#### 2. Inconsistent Build Context for `metadata-action`
- **Source:** GEMINI
- **Severity:** MEDIUM
- **Finding:** While the spec correctly identifies the `images/` directory as the build context for `docker/build-push-action`, it does not explicitly state how `docker/metadata-action` will be configured to handle multiple images in a matrix. If not handled carefully, metadata like `org.opencontainers.image.title` might be generic or incorrect for specific images.
- **Recommendation:** Ensure `docker/metadata-action` is called within the matrix and explicitly set the image name for each iteration (e.g., `images: veesix/bngtester-${{ matrix.image }}`).

### LOW

#### 3. No Build Caching Strategy
- **Source:** GEMINI
- **Severity:** LOW
- **Finding:** The spec does not mention Docker layer caching (e.g., `cache-from: type=gha`, `cache-to: type=gha`). While the current images are small, build times will increase as the project grows or more complex images are added.
- **Recommendation:** Add GitHub Actions cache configuration to the `docker/build-push-action` step to speed up subsequent builds.

#### 4. Missing OCI Labels and Metadata
- **Source:** GEMINI
- **Severity:** LOW
- **Finding:** The spec doesn't explicitly mention adding standard OCI labels (e.g., `source`, `description`, `licenses`) to the images. This makes the images less discoverable and harder to audit on Docker Hub.
- **Recommendation:** Use `docker/metadata-action` to automatically inject labels such as `org.opencontainers.image.source` and `org.opencontainers.image.description`.

#### 5. `latest` Tag Race Condition
- **Source:** GEMINI
- **Severity:** LOW
- **Finding:** The spec states that both pushes to `main` and `v*` tags will update the `latest` tag. If a tag is pushed shortly after a push to `main` (or vice-versa), there is a slight race condition on which image becomes `latest` on Docker Hub.
- **Recommendation:** This is usually acceptable for a new project, but consider if `latest` should strictly follow the `main` branch or the highest semver tag. For now, the proposed behavior is standard but should be monitored.

## Conclusion

The spec is well-defined and covers the essential requirements. Addressing the `pull_request` trigger and adding build caching will significantly improve the developer experience and reliability of the pipeline.

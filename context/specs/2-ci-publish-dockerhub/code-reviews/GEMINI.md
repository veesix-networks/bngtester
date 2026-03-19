# Code Review: CI Pipeline to Publish Subscriber Images (Gemini)

## Summary Checklist

- [x] **GitHub Actions Best Practices:** Uses standard actions, versioned tags (`@v4`, `@v5`, `@v6`), and clean job separation.
- [x] **Docker Build-Push Configuration:** Correctly uses `buildx`, handles shared build context, and implements build-before-push logic.
- [x] **Security:** Secrets are handled via GitHub Secrets; `pull_request` trigger correctly excludes pushing.
- [x] **Workflow Trigger Correctness:** Triggers on `main`, `v*` tags, and `pull_request`.
- [x] **Caching Efficiency:** Uses GHA cache with per-image scopes and `mode=max`.
- [x] **OCI Metadata Completeness:** Labels are injected via `metadata-action`.

## Findings

### MEDIUM

#### 1. Tagging Strategy Drift from Spec
- **Source:** GEMINI
- **Severity:** MEDIUM
- **Finding:** The implementation's `metadata-action` configuration only enables the `latest` tag on the default branch (`enable={{is_default_branch}}`). However, the [Implementation Spec](../IMPLEMENTATION_SPEC.md#tag-strategy) explicitly requires: "On semver tag `v*`: Tag as the semver version (e.g., `v0.1.0` → `0.1.0`), **plus `latest`**."
- **Impact:** Release tags will not update the `latest` pointer on Docker Hub, which may confuse users who expect `latest` to represent the most recent stable release.
- **Recommendation:** Update the `metadata-action` `tags` block to include `type=raw,value=latest` for semver tags, or adjust the logic to ensure releases update `latest`.

### LOW

#### 2. Redundant Metadata and Build Steps in `push` Job
- **Source:** GEMINI
- **Severity:** LOW
- **Finding:** The `push` job completely duplicates the `metadata-action` and `build-push-action` configuration from the `build` job. While the GHA cache (`cache-from: type=gha`) ensures this is fast, it increases the maintenance surface area of the workflow file.
- **Impact:** Changes to build arguments, contexts, or tagging logic must be manually synchronized between two jobs.
- **Recommendation:** Consider using a [reusable workflow](https://docs.github.com/en/actions/using-workflows/reusing-workflows) if the project grows, or accept the duplication as a tradeoff for the "build-all-then-push-all" safety property.

#### 3. Missing Standard OCI Annotations
- **Source:** GEMINI
- **Severity:** LOW
- **Finding:** While labels are enabled, the `metadata-action` is not configured with explicit values for `org.opencontainers.image.source` or `description`.
- **Impact:** Docker Hub images will lack helpful metadata pointing back to this repository.
- **Recommendation:** Add `annotations` or `labels` to `metadata-action` to include the repository URL and a brief description per image.

#### 4. Hardcoded Registry and Image Name Prefix
- **Source:** GEMINI
- **Severity:** LOW
- **Finding:** The image prefix `veesix/bngtester-` is hardcoded in both the `build` and `push` jobs.
- **Impact:** If the organization or naming convention changes, multiple updates are required.
- **Recommendation:** Define the image prefix as a top-level `env` variable in the workflow.

## Conclusion

The implementation is high quality, robust, and follows modern CI/CD patterns for Docker. The dynamic image discovery is a significant improvement over a static list. The only notable issue is the drift in the `latest` tagging logic for releases.

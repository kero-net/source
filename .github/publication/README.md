# Publication

SCOPE uses Visit-Counter's source-to-channel publication model. The `source`
branch is canonical; `canary`, `beta`, and `stable` are generated from immutable
source commits. `.github/publication/config.yml` is the canonical payload allowlist.

Generated manifests bind each channel payload to its source commit and release
identity. Fix publication errors in canonical source or tooling and regenerate;
never patch a generated branch manually.

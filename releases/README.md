# Release Records

KERO release records are authored only after an approved immutable release
identity exists. Distribution uses the `canary`, `beta`, and `stable` channels;
the channel does not replace the release identity.

Release record filenames use `YYYY.MM.N-KIND.md`.

Every authored record must also appear under the `version` choices in
`.github/workflows/release.yml`. Repository validation enforces that the
dropdown and `releases/records/` contain exactly the same release IDs.

Publication uses the organization-owned Frogge KERO GitHub App and release
signing identity. The source repository receives them through these
organization Actions values:

- Variable `KERO_RELEASE_APP_CLIENT_ID`.
- Secret `KERO_RELEASE_APP_PRIVATE_KEY` containing the complete PEM.
- Variable `KERO_GPG_KEY_ID` containing the full 40-character fingerprint.
- Secrets `KERO_GPG_PRIVATE_KEY` and `KERO_GPG_PASSPHRASE`.

The workflow mints a short-lived token explicitly restricted to the `kero`
repository. Generated branches, signed tags, and GitHub Releases are published
to `kero-net/kero`.

Channel branches and GitHub Release objects are replaceable. Release tags are
immutable: rerunning publication keeps an existing verified tag, replaces the
selected channel branch, deletes only the old GitHub Release object, and then
recreates that Release from the authored record. GitHub's automatic source
archives remain available; the publisher does not attach a duplicate tarball.

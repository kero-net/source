# Release Records

KERO release records are authored only after an approved immutable release
identity exists. Distribution uses the `canary`, `beta`, and `stable` channels;
the channel does not replace the release identity.

Release record filenames use `YYYY.MM.N-KIND.md`.

Every authored record must also appear under the `version` choices in
`.github/workflows/release.yml`. Repository validation enforces that the
dropdown and `releases/records/` contain exactly the same release IDs.

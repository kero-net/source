# Release Records

This directory contains authored Markdown used to prepare GitHub Releases. It
exists only on `source`; publication branches and repository payloads must not
include these files.

Release record filenames use `YYYY.MM.N-KIND.release.md`. The release ID uses a
monthly sequence and one of `regular`, `hotfix`, or `security`, for example
`2026.08.1-regular.release.md`.

Each record must contain a matching first heading and non-empty Summary,
Notable Changes, Issues Addressed, Compatibility, and Verification sections.
Publication prepends the record to GitHub's generated release notes. Security
records must not disclose private vulnerability details.

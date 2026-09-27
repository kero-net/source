# Release repository

`kero-net/kero` is the public release repository. It owns generated `canary`,
`beta`, and `stable` channel branches, release packages, issues, discussions,
and release-facing documentation.

Channel branches are generated surfaces and must not be hand-edited. Their
schema and provenance are defined by the publication contract under
[`../publication/`](../publication/).

The repository also owns KERO's repository-view counter. Its scheduled workflow
writes counter assets to the `visit-counter` branch; organization presentation
may embed those assets but does not collect an independent counter.

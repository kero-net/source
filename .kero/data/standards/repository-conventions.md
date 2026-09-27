# Repository conventions

- Keep implementation facts in the repository that owns the implementation.
- Register durable product, project, or standards knowledge under `.kero/data/`
  before relying on it as a KERO contract.
- Follow the semantic tree rules in [`knowledge-tree.md`](knowledge-tree.md).
- Use roadmap files and checklists for progress and evidence, not for canonical
  product truth.
- Keep disposable generated artifacts under `.heap/` or KERO `.runtime/`; do
  not commit either location.
- Use `kero-net/.github` for organization defaults. Keep repository-specific
  workflow triggers and configuration in the owning repository.
- Change source, release, and organization repositories through their own Git
  histories; `kero-net/` itself remains only a local coordination container.

## Why

These rules keep ownership visible and prevent generated state, temporary work,
or organization-wide defaults from becoming indistinguishable from the source
that actually defines KERO behavior.

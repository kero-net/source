# Entry Points

Entry points own startup or boundary wiring.

They may coordinate other modules, connect to engine/plugin lifecycle, validate
environment assumptions, and call into project-owned surfaces.

Entry points should not become dumping grounds for implementation behavior that
belongs in a structure-specific module.

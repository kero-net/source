# Luau System

Luau source does not follow one universal file template.

A file first chooses a structure based on its responsibility. Root language
rules apply across structures. File/tree rules describe how modules compose.
Structure rules describe the whole-file shape for a specific script family.

```text
luau/
  language/
    naming.md
    types.md
    functions.md
    expressions.md

  files/
    file-system.md
    modules.md
    init-files.md
    entry-points.md
    generated-files.md

  structures/
    structure-system.md
    oop/
    ecs/
    functional/
    procedural/
```

When adding or editing Luau, choose in this order:

1. source responsibility
2. file/tree role
3. structure family
4. language-wide naming/type/function/expression rules
5. structure-specific required and absent elements

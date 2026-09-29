# ECS System

ECS is a recognized Luau structure family.

Detailed ECS rules should be defined from real source once a repository adopts
ECS. Until then, do not invent validator rules for ECS.

ECS docs are placeholders for future concrete ownership, not a claim that every
repository already uses ECS.

The point of ECS is to separate data shape from behavior over many entities.
Components describe what an entity has. Systems describe what happens to
entities with selected components. Queries describe how those entities are
selected.

Use ECS when the useful question is:

```text
Which entities have this data, and what behavior runs over that set?
```

Do not choose ECS just because a module stores records. A plain data table,
procedural module, or OOP object is usually clearer until entity/component
iteration is the actual pressure.

Until real ECS code exists in an adopting repository, examples should stay
sketches. They may show responsibility boundaries, but they must not define
final APIs, validator rules, or framework choices.

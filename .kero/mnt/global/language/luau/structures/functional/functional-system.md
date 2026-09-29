# Functional System

Functional modules own one operation or a small composition of pure operations.

Use this structure when a table identity, lifecycle, constructor, or module
state would be ceremony.

The point is to make the operation itself the module boundary. Callers require
the module because they need the transformation, not because they need a named
namespace.

Use this for small, stable operations where the useful question is:

```text
What value comes out for these inputs?
```

Do not use this when the file needs multiple public operations, durable
module-scoped state, setup/teardown, or an instance lifetime. Those pressures
belong to procedural or OOP structures.

# Luau Structure System

A Luau file chooses a structure before applying structure-specific rules.

Language-wide rules still apply, but each structure decides which elements are
required, optional, or intentionally absent.

The structure is chosen by why the file exists, not by which syntax looks
familiar.

| Why the file exists | Structure |
| --- | --- |
| Transform inputs into one result without owned state | functional return-function module |
| Group related operations behind one named module surface | procedural table module |
| Manage independent instances with identity, lifetime, or owned resources | OOP object |
| Expose declarative values that callers read as data | data table module |
| Own startup wiring, boundary normalization, or subsystem facade behavior | entry point |
| Describe data attached to entities without per-instance methods | ECS component |
| Iterate over queried entities and apply behavior from their data | ECS system |

Do not add ceremony from another structure unless the current file earns it.

If a file feels awkward, check the "why" before changing syntax. A table module
with one function may want to become a return-function module. A data table with
private mutable state may want to become procedural. A procedural module that
manufactures independent lifetimes may want to become OOP.

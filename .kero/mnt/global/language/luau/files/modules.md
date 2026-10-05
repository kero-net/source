# Luau Modules

Modules should expose the shape that matches their responsibility.

Do not wrap a single operation in an identity table just to make every file look
the same.

Common module shapes:

- return-function module
- table module
- metatable-backed object
- data/config module
- entry point
- ECS component or system

The structure docs own the whole-file expectations for each shape.

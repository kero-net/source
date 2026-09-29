# OOP System

OOP rules apply to metatable-backed objects with independently constructed
instances.

Use OOP when a value has identity, lifetime, owned resources, or instance
methods that operate on stored fields.

The point is to make ownership explicit. An object says: this value owns state
or resources over time, and methods operate on that particular instance.

Use this when the useful question is:

```text
Which instance owns this state or resource?
```

Do not use OOP for one pure operation, static grouped functions, plain data, ECS
components, or startup wiring.

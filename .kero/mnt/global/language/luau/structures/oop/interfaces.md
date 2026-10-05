# OOP Interfaces

Shared abstractions should use explicit structural contracts.

Do not use a metatable-derived concrete instance type as a shared
cross-implementation interface.

```luau
export type Disposable = {
	Destroy: (self: Disposable) -> (),
}
```

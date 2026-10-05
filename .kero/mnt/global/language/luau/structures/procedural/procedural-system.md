# Procedural System

Procedural modules group related operations without independent constructed
instances.

Use this for stateless grouped behavior or module-scoped state that does not
want OOP identity.

The point is to name a public surface without claiming every caller gets its own
object. Callers require the module because they need a toolbox for one domain.

Use this when the useful question is:

```text
Which operation in this domain should I call?
```

Do not use this when the module is only literal data, when one function would
carry the whole file, or when each caller needs an independent lifetime with
owned resources.

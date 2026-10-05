# Language System

Language rules are owned by language and concept, not by a generic style bucket.

Use this shape:

```text
/mnt/data/documents/.scope/knowledge/language/<language>/<concept>.md
```

Examples:

- `/mnt/data/documents/.scope/knowledge/language/luau/files/file-system.md`
- `/mnt/data/documents/.scope/knowledge/language/luau/language/naming.md`
- `/mnt/data/documents/.scope/knowledge/language/luau/structures/oop/objects.md`
- `/mnt/data/documents/.scope/knowledge/language/luau/language/types.md`

Different languages may have different concepts, naming systems, exception
rules, file shapes, generated-code boundaries, and validator support.

Do not force a concept into every language just because another language has it.
Add a concept owner when the language actually needs one.

## Ownership

Language docs own language-specific policy.

Validator scripts may enforce language policy, but they do not define it.

Global language style may eventually be synced across a wider workspace. Until
shared sync exists, each repository owns its local adoption notes.

## Exceptions

Exception policy belongs to the language concept being excepted.

Exception implementation belongs to the script, config, or tool that applies the
exception.

For example:

- `/mnt/data/documents/.scope/knowledge/language/luau/language/naming.md` owns whether generated Luau may use a
  naming exception.
- the repository validator script owns how that exception is detected.
- repository validator metadata owns implemented rule metadata.

An exception must name:

- the rule or concept being excepted
- the owner that permits it
- the scope where it applies
- the evidence or script behavior that recognizes it

Do not create ad hoc exceptions inside agent replies or disposable plans.

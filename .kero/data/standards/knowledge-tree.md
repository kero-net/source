# KERO knowledge tree standard

KERO durable knowledge is organized as a semantic tree. The filesystem should
represent concept inheritance, not merely group documents by topic or document
type.

## Node representation

A concept that has children is a directory whose `README.md` contains the
knowledge owned by that concept:

```text
distribution/
├── README.md
├── windows/
│   ├── README.md
│   ├── x64.md
│   └── arm64.md
```

A concept with no children is a named Markdown file:

```text
windows/
├── x64.md
└── arm64.md
```

Therefore:

- **branch node** = directory + `README.md`;
- **leaf node** = named `.md` file.

If a leaf later gains children, promote it without changing its conceptual
identity:

```text
arm64.md
```

becomes:

```text
arm64/
├── README.md
├── toolchain.md
└── packaging.md
```

## Inheritance rule

A node contains only knowledge that is true for that node and may be inherited
by every descendant beneath it.

For example:

```text
distribution/README.md
```

contains rules shared by every distribution target;

```text
distribution/windows/README.md
```

contains only rules shared by every Windows target; and

```text
distribution/windows/arm64.md
```

contains facts specific to Windows ARM64.

Do not copy a child-specific fact into an ancestor for convenience. Do not
repeat an ancestor fact in every child. Link to the authoritative node when a
reader needs related context.

## Directory rule

A directory exists because its concept has children and owns meaningful shared
knowledge. Do not create a directory only to make the tree look symmetrical or
to act as a generic filing category.

If a directory would contain only a `README.md` and has no actual child
concepts, prefer a leaf `.md` file unless there is a concrete reason for the
concept to be a branch now.

## Decision rule

Decisions live with the concept they constrain. There is no separate
`decisions/` tree for canonical product truth.

For a non-obvious decision, record enough context for a future contributor to
understand not only what was chosen but why it was reasonable. Prefer this
shape when applicable:

```md
## Decision

What is currently chosen.

## Why

The constraints or goals that caused the decision.

## Consequences

Important costs, requirements, or behavior created by the decision.

## Reconsider when

A concrete condition that would make the decision worth revisiting.
```

Not every document needs all four headings. The requirement is that a durable
constraint must not survive as an unexplained commandment when its rationale is
known.

## Dynamic notes and replacement

A dynamic note records active reasoning, investigation, or work that can
change. It belongs at the narrowest product or project concept it concerns; it
does not belong in a generic chronological archive, `.runtime/`, or a sibling
tree that obscures its subject.

Name the file for that subject, not for when it was created. Do not use dated
filenames such as `2026-09-22-plan.md`. A date inside a document is appropriate
only when it explains a material event.

Current truth is replaced in place. When a new decision or plan supersedes an
older one, the current canonical document must say what it replaces, why the
change was made, and its consequences. A retained former document must be
marked **Superseded** and link directly to the document that replaced it. This
preserves continuity without making stale alternatives appear current.

## Product truth versus project state

Canonical product behavior belongs under `data/product/`. Implementation
progress, temporary migration state, build workflow, and roadmap status belong
under `data/project/`.

A roadmap stage references the product concepts it implements. It does not
restate their contracts. This keeps completed or reordered work from changing
where contributors look for the current truth.

## Cross-cutting relationships

Some concepts describe a relationship between siblings rather than fitting
entirely beneath one of them. Put such a document at their nearest meaningful
common ancestor.

For example, the core/host interface belongs under the execution concept rather
than being duplicated under both the portable core and native host branches.

## Naming

Use short, descriptive lower-case names with hyphens where needed. Names should
identify the concept, not the document type. Avoid organizational buckets such
as `misc`, `others`, or `decisions` when a semantic parent exists.

`README.md` is reserved for branch-node knowledge. Leaf concepts use their own
names.

# Reversible representation research

## Status

This is KERO's active research direction for a future compiled representation.
It is not an implemented feature, a permanent on-disk format, or a change to
the current local processing and trust contract.

## Research question

Given a source corpus `S`, KERO investigates whether it can produce a
representation `R = (G, P)` such that:

```text
Execute_R(G, P) = S
```

`R` is a fixed, versioned KERO reconstruction runtime. `G` is a bounded
reconstruction program or transform description, and `P` is its residual
payload. Exact mode requires byte-for-byte equality with the source corpus.

The runtime is shared infrastructure and does not count toward a particular
representation's size. Every representation-specific byte does count: program
instructions, transform versions, parameters, dictionaries, references,
metadata, and payload. The primary size cost is therefore `|G| + |P|`.

KERO does not seek an uncomputable optimum over arbitrary reconstruction
programs. It investigates bounded composition and selection over known,
reversible transforms executed by a constrained reconstruction runtime.

## Guarantees

The following modes are distinct and must never be presented as interchangeable:

- **Exact lossless** reconstructs the original source bytes exactly. This is
  the first experimental guarantee.
- **Structural equivalence** reconstructs a defined equivalent structure and
  requires its own format-specific equivalence contract.
- **Semantic/query preservation** preserves an explicitly defined query or
  semantic contract and may not preserve source bytes or source structure.

Only exact lossless reconstruction belongs in the initial prototype. Structural
and semantic modes are later research, not relaxed interpretations of exact
mode.

## Reconstruction runtime

The experimental runtime is a small deterministic interpreter, not a container
for arbitrary executable decoder code. Each operation must be finite,
deterministic, bounds-checked, versioned, reversible in exact mode, and fully
described by representation-specific bytes.

Initial experiments may compose qualitatively different operations such as raw
payload, established general-purpose codecs, content references, deltas, shared
dictionaries, and concatenated regions. The operation set and temporary
serialization are experimental and disposable; they do not select a permanent
KERO instruction set or physical representation format.

Established codecs are potential runtime primitives and mandatory comparison
baselines, not algorithms KERO intends to replace. Initial baselines are raw
data, DEFLATE, and Zstandard. KERO's research value is the compact,
deterministic composition and selection of transformations across a corpus.

## Corpus and selection

The corpus, rather than only an individual file, is the representation scope.
Experiments may discover repeated fragments, similar files, shared dictionary
material, deltas, and mixed regional representations. They compare per-file
compression, whole-corpus compression, and composed corpus representations.

Candidate selection begins with bounded enumeration: encode each legal
candidate, measure its actual serialized representation cost, and retain valid
results. Later work may investigate partitioning, shared-substructure discovery,
dynamic programming, beam search, graph search, and cost heuristics only when
benchmark evidence warrants them. Results report a Pareto frontier rather than
combining size, encode time, decode time, and memory into an invented score.

The first benchmark corpus is small, checked in, license-reviewed, and covers
text, source code, and structured data. It records exact reconstruction,
program bytes, payload bytes, total bytes, encode time, decode time, tracked
peak heap use, and determinism. Random access is initially reported as
unsupported rather than simulated by an undefined access contract.

## Boundaries

LLM-driven representation and model dependencies are excluded from this
research track and its initial prototype. This is not a permanent project-wide
prohibition on future machine-learning or language-model research.

The research does not yet introduce a Rust crate, public core API, service or
CLI command, Qt capability, persistent `.kero` state, MCP integration, remote
service, semantic retrieval, or PDF/OCR/video ingestion. Prototype contracts
and serialization are disposable. Successful implementation components may be
promoted only after benchmark evidence supports a stable product contract.

Current `.kero/data`, `mnt`, `config`, and `.runtime` contracts remain in force.
The possibility that a later `.kero` representation behaves as compiled machine
state is under investigation; it does not decide storage shape, migration, or
ownership rules today.

## Reconsider when

Reconsider this direction after repeatable benchmark evidence shows whether
bounded reconstruction composition provides useful size or capability tradeoffs
beyond established codecs. Any production format, runtime operation, storage
location, or non-exact guarantee requires a separate accepted contract.

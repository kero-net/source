# Implementation roadmap

Roadmap files describe implementation status and evidence. They reference the
canonical product concepts they implement rather than restating those
contracts.

| Stage | Status |
|---|---|
| [00 — foundation](stage-00-foundation.md) | complete |
| [01 — structured configuration](stage-01-structured-configuration.md) | complete |
| [02 — WASM core and host ABI](stage-02-wasm-host-abi.md) | complete |
| [03 — environment lifecycle](stage-03-environment-lifecycle.md) | complete |
| [04 — repository and mounts](stage-04-repository-and-mounts.md) | complete (local contract) |
| [05 — knowledge input](stage-05-knowledge-input.md) | complete (local snapshot contract) |
| [06 — processing and trust](stage-06-processing-and-trust.md) | complete (local contract) |
| [06.5 — filesystem compatibility](stage-06.5-filesystem-compatibility.md) | complete (portable local snapshot contract) |
| [07 — terminal service](stage-07-terminal-service.md) | in progress |
| [08 — Qt visual client](stage-08-qt-visual-client.md) | queued after stage 07 |
| [09 — application and CLI integration](stage-09-application-and-cli-integration.md) | planned |
| [10 — functional testing](stage-10-functional-testing.md) | planned |
| [11 — application and CLI refinement](stage-11-application-and-cli-refinement.md) | planned |
| [12 — cross-platform proof](stage-12-cross-platform-proof.md) | planned |
| [13 — final validation and publication](stage-13-final-validation-and-publication.md) | planned |

## Separate research track

[`Representation-machine research`](../representation-machine-research.md) is
an active non-delivery track. It does not renumber or replace Stage 08, which
remains the Qt visual client, and may create later numbered stages only after
benchmark evidence supports a product implementation contract.

## Current sequence

Stage 06.5 hardened the established snapshot boundary against filesystem
differences before Stage 07 exposes it through a persistent service and terminal
interface. Stage 08 places the Qt client above that service. Stage 09 connects
the application and CLI into one coherent product surface; stage 10 proves the
completed behavior; and stage 11 refines those interfaces from that evidence.
Only then do stages 12 and 13 prove the supported native platforms and prepare
publication.

## Replacement record

Stages 09 through 13 replace the former single "cross-platform release" stage.
Integration, functional testing, and interface refinement now precede platform
proof because an installer cannot demonstrate unfinished application behavior.
The consequence is that cross-platform packaging remains important, but it no
longer interrupts the service and client delivery sequence.

## Why

Progress is temporal; product truth is durable. Keeping roadmap state in the
project branch means completing, splitting, or reordering a stage does not move
the authoritative explanation of how KERO works.

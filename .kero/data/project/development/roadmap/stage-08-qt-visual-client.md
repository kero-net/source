# Stage 08 — Qt visual client

**Queued after stage 07.** Build the Qt Widgets client above the
terminal-capable service.

The client presents real service state and operations for repository status,
mounting, knowledge, processing, and trust. It must not own or duplicate the
underlying behavior. Visual refinement is deliberately deferred to stage 11,
after application and CLI integration and functional testing establish which
workflows need refinement.

## Completion evidence

- The client connects to and operates only through the stage-07 service
  boundary.
- Core repository and knowledge workflows are usable from the client.
- Service failures and disconnected states have clear, recoverable UI states.

Stage 09 joins the application and CLI surfaces once both entry points work.

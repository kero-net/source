# Private toolchain adapters

These adapters are implementation details of `distribution/actions/kero-build`.
They are invoked only by the typed distribution target coordinator; contributors
must use the single public `kero-build` entry point instead of running them.

The build may download and extract tools into `.heap/build/toolchains`, but it
must never generate source scripts there.

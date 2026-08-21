# Manual page source

`kero.1` is the canonical roff source for the section 1 manual page. Keeping an
uncompressed roff file under `docs/man/` makes its documentation ownership
explicit while preserving the conventional source format and
makes reviews and patches straightforward. Release packages may compress the
installed copy as `kero.1.gz`; the repository source should remain plain text.

Run `groff -z -mandoc docs/man/kero.1` to lint the document without rendering it, or
`man -l docs/man/kero.1` to preview it locally. Command syntax is owned by Clap in the
Rust CLI; the manual adds conceptual and operational guidance rather than trying
to duplicate every `--help` line.

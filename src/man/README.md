# Manual page source

`kero.1` is the canonical roff source for the section 1 manual page. Keeping an
uncompressed roff file under `code/man/` keeps command documentation beside the
CLI while preserving the conventional source format and
makes reviews and patches straightforward. Release packages may compress the
installed copy as `kero.1.gz`; the repository source should remain plain text.

Run `groff -z -mandoc code/man/kero.1` to lint the document without rendering it, or
`man -l code/man/kero.1` to preview it locally. Command syntax is owned by Clap in the
Rust CLI; the manual adds conceptual and operational guidance rather than trying
to duplicate every `--help` line.

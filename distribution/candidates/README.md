# Distribution candidates

`lua distribution/scripts/local-run.lua propose` writes one TOML manifest here
for a clean, committed source revision with complete local native evidence.

The manifest is reviewed in the source repository. Generated packages, logs,
and local evidence remain ignored under `.heap/`; maintainers reproduce an
approved candidate locally before signing or publishing it.

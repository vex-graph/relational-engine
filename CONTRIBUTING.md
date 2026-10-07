# Contributing

Read the complete [workspace preferences.md constitution](https://gist.github.com/vex-graph/4132a6c45cb6d3797c3e8eff2e94035a)
before work. In the integrated workspace it is the real workspace-root
`preferences.md`, not a file to duplicate in this repository.

The Git Workflow Law, Feature Implementation and Adversarial Proof Law,
Living Documentation Law and Test Segregation Law apply. Preserve imported
reference code until its replacement has an explicit contract and proof.

`src/` and `rust/` are parallel scratchpads, not two production engines.
CMake is CLion-only metadata. Use b/native compiler tooling for actual work;
do not make the runtime depend on CMake or compile out real diagnostics.

The repo-local lawbook is an explicit reading gap at this scaffold stage.
Public scratchpad code is not battle-tested readiness. Owner tests live in the
independent workspace `tests/relational-engine/` checkout; generated binaries,
Cargo target output and IDE state stay ignored.

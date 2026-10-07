# C primitive search

Reserved for native C search over explicitly borrowed, length-bounded primitive
spans. Rust owns storage; C never frees it or assumes Rust atomic layout. Search
requires stable input for the call; GPU work completes before CPU access.

No search implementation is shipped here yet. Existing `src/relational/` and
other imported C directories remain comparison material, not the new runtime.
Future owner tests mirror this directory under `tests/relational-engine/search/primitives/`.

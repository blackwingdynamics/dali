# Adding a target

Contributors should follow this order:

1. Add and validate `targets/<profile>.toml`.
2. Add a backend selected by a Cargo feature and keep it behind the platform
   facade.
3. Implement reset, clock, GPIO, logging, storage, USB, memory, and protection
   behavior supported by the target.
4. Connect the backend resources to the existing core interfaces without
   changing AMRN or ABI policy accidentally.
5. Add manifest, host, and embedded-target validation; add hardware evidence
   for every claimed physical behavior.
6. Document unsupported capabilities and update the roadmap before expanding
   the compatibility contract.

Adding a target must not require editing unrelated core policy modules. If it
does, the boundary is incomplete and should be refined before adding more
boards.

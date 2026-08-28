# Documentation Quality and Enterprise Readiness

This roadmap defines the documentation work required to make Dali OS easier
to evaluate, adopt, operate, and maintain at production and enterprise level.
It is documentation-first work and must not change runtime behavior, hardware
contracts, frozen subsystem boundaries, or security claims without separate
approval.

## 1. Documentation inventory and ownership

- [x] Maintain one canonical `README.md` index for each documentation domain.
- [x] Verify that every document has one clear owner, scope, and source of truth.
- [x] Remove stale paths, duplicate guidance, and contradictory claims.
- [x] Keep `docs/file-structure/repository-tree.md` synchronized with the real
  repository.

## 2. Terminology and writing consistency

- [x] Define a project-wide terminology and naming guide.
- [x] Standardize terms for targets, profiles, backends, manifests, phases,
  evidence, guarantees, and limitations.
- [x] Apply consistent heading hierarchy, link style, command formatting, and
  status vocabulary.
- [x] Keep all code, logs, errors, and documentation text in English.

## 3. Automated documentation validation

- [x] Add an internal-link checker for Markdown files.
- [x] Validate that every documented file and directory exists.
- [x] Validate roadmap indexes and repository-tree entries against the actual
  repository structure.
- [x] Run documentation checks in CI and preserve actionable failure output.
- [x] Keep formatting and whitespace validation compatible with existing hooks.

## 4. Requirements and evidence traceability

- [x] Map major requirements to implementation modules, tests, documentation,
  and hardware evidence.
- [ ] Separate host validation, embedded compilation, flashing, and Silicon
  Trace evidence in every acceptance record.
- [ ] Record board, wiring, firmware revision, power source, transport,
  expected trace, observed trace, and limitations for physical claims.
- [ ] Ensure incomplete hardware validation cannot be presented as completed
  functionality.

## 5. CLI and SDK documentation

- [ ] Complete CLI installation, command, option, output, error, and workflow
  references.
- [ ] Document SDK contracts, service boundaries, version compatibility, and
  supported application patterns.
- [ ] Add reproducible examples for package creation, inspection, signing,
  installation, and device interaction.
- [ ] Document unsupported modes and failure recovery without inventing
  hardware behavior.

## 6. Security, operations, and contribution guidance

- [ ] Document threat-model scope, security guarantees, non-guarantees, and
  evidence requirements in one navigable structure.
- [ ] Add security disclosure and incident-response procedures.
- [ ] Document onboarding, development setup, release preparation, rollback,
  and troubleshooting workflows.
- [ ] Define review expectations for architecture, unsafe code, hardware
  changes, documentation claims, and generated artifacts.

## 7. Release and maintenance readiness

- [ ] Define documentation versioning and compatibility expectations.
- [ ] Require documentation updates for public behavior, contract, evidence,
  and security-claim changes.
- [ ] Add a release documentation checklist covering links, examples, known
  limitations, and migration notes.
- [ ] Review the documentation set before startup-funding or external technical
  evaluation submissions.

## Completion criteria

This roadmap is complete when the documentation passes automated link and
structure checks, major contracts have traceable validation and evidence, CLI
and SDK workflows are reproducible, security boundaries are explicit, and an
external engineer can navigate the repository without relying on undocumented
context.

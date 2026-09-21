# Security Disclosure and Incident Response

This document defines the handling process for suspected security issues in
Dali OS. It is a project procedure, not a claim that every listed response
capability is currently staffed or automated.

## Responsible disclosure

Do not publish exploit details, credentials, private keys, target dumps, or
reproducible attack material in a public issue or pull request. Report a
suspected vulnerability through the project's private maintainer channel. If
no private channel is available, contact the project owner before public
disclosure and provide only the minimum detail needed to establish a secure
conversation.

A useful report includes:

- affected revision, target, feature set, and build profile;
- affected component and the security boundary it crosses;
- concise impact statement and attack preconditions;
- reproducible steps or a minimal non-sensitive proof;
- host, embedded, and Silicon Trace evidence separately;
- suspected workaround and whether data, keys, or devices may be exposed.

Do not include live credentials, private signing material, undisclosed device
identifiers, or unrelated user data.

## Triage

The maintainer first confirms receipt through the same private channel, assigns
an internal tracking reference, and classifies the report by affected
boundary, exploitability, and evidence quality. A report remains unconfirmed
until the implementation and the relevant tests or target traces support the
claim.

The triage record should state:

- affected versions and target profiles;
- whether the issue is baseline, feature-gated, or documentation-only;
- whether the issue crosses a privilege, memory, storage, cartridge, or
  hardware boundary;
- reproducibility and required physical setup;
- temporary containment and release impact.

## Containment and remediation

Until a fix is reviewed, affected acceptance claims must be narrowed or marked
unverified in the owning documentation. Do not hide the issue by disabling a
test, changing a target profile silently, weakening validation, or adding a
temporary timing workaround.

Remediation must:

1. identify the owning module and contract;
2. preserve frozen subsystem boundaries unless a separate architecture review
   approves a change;
3. add or update host coverage for hardware-neutral behavior;
4. add embedded and Silicon Trace evidence when physical behavior is involved;
5. update security scope, limitations, versioning, and roadmap records;
6. use a focused reviewed commit and record the remaining limitations.

## Disclosure decision

Public disclosure timing is decided after the affected versions, remediation
status, user impact, and available mitigation are understood. The disclosure
must describe the affected boundary, impact, fixed revision or mitigation,
validation level, and any remaining hardware evidence gap. It must not claim a
security guarantee that the implementation and evidence do not support.

## Incident records

Incident records must not contain secrets or unnecessary personal/device data.
Retain only the evidence needed to reproduce, review, remediate, and audit the
decision. Link the final public explanation, if one is issued, to the relevant
security and versioning documents.

# 09 — Baseline Regression Isolation and Recovery

This work track preserves the last proven F405 firmware baseline and isolates
future changes one at a time. No checkpoint is complete from compilation alone;
each firmware change requires the documented F405 flash and console evidence.

## Proven baseline

- [ ] Create an explicit known-good F405 baseline checkpoint and record its
  exact firmware, hardware, and console evidence before attempting another
  change.
- [x] Build the release firmware for `thumbv7em-none-eabihf`.
- [x] Flash and verify the image on the STM32F405 using SWD.
- [x] Confirm `Reset cause: Software`, SDIO initialization, repository loading,
  AMRN validation, signature verification, and one `Hello World from AMRN`
  application log on the USB CDC console.
- [ ] Preserve this baseline with an explicit Git commit or tag and record the
  image and hardware evidence.

## Isolate the regression

- [ ] Reintroduce only the streaming parser refactor and run the complete F405
  build, flash, reset, and USB CDC regression cycle.
- [ ] Revert the parser refactor and reintroduce only the watchdog reset-cause
  change; run the same F405 regression cycle.
- [ ] Compare the two isolated results with the proven baseline and identify
  the first change that reproduces the failure.
- [ ] Do not combine changes again until each isolated change has passed its
  own hardware regression.

## Implement verified fixes

- [ ] Replace any reset-cause heuristic with an F405-specific interpretation
  that preserves genuine watchdog Safe Mode behavior and handles co-latched
  reset flags safely.
- [ ] Return the streaming parser refactor only after semantic tests and F405
  hardware evidence pass without changing the loader contract.
- [ ] Add bounded timeout handling to CLI probe discovery so `dali device list`
  cannot block indefinitely when `probe-rs list` is unresponsive.
- [ ] Add a precise SDIO initialization diagnostic for `Unsupported` failures,
  identifying the rejected protocol stage without exposing sensitive data.

## Close-out

- [ ] Run the complete validation suite after the isolated fixes are merged.
- [ ] Create separate Conventional Commits for each verified fix.
- [ ] Update hardware evidence, testing documentation, file structure, and
  roadmap status to match only the observed F405 results.

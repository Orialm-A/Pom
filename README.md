# Pom

**Pom** is a small CLI tool to scaffold and organize embedded C projects.

It focuses on **structure, layering and long-term maintainability** rather than flashy automation.

> Yes, *Pom* also stands for *Polyoxymethylene*.
> If you have −10 diopters in each eye, you might notice similarities:
> both are used where precision, rigidity and low friction matter. 🙂

------

## ⚠️ Project Status

Pom is currently in **alpha**:

- Only two commands are implemented:
  - `pom project create`
  - `pom module add`
- There is **no installation or distribution mechanism yet**
- It is not intended for external usage at this stage
- The content of the next release is not decided

This repository mainly documents ongoing development.

------

## Current CLI Overview

```bash
pom project create [NAME] [--path PATH] [--target TARGET] [--dry-run]
pom module add [NAME] [--layer LAYER] [--brief TEXT] [--details TEXT] [--dry-run]
```

Other commands (`build`, `flash`, `monitor`, `clean`, etc.) exist in the CLI structure but are not implemented yet.

------

## Philosophy

Pom is built around a few ideas:

- Embedded-first workflow
- Clear separation of layers
- Deterministic project structure
- Minimal magic

It is primarily designed to replace personal Bash scripts used to bootstrap embedded projects.

------

## Supported Targets (Planned)

- STM32 Nucleo F411RE
- ESP32-C3-DevKit-C
- Raspberry Pi Pico W

Support is driven by available hardware and real usage.
Devkits are currently considered as targets because generated files may differ between two boards using the same MCU. However, nothing in the implementation prevents defining MCUs directly as targets in the future.

------

## Development Approach

Pom evolves incrementally through small MVPs:

- Clean CLI
- Target project generation
- Module management
- Target abstraction
- Build command unification

Features are implemented only after dogfooding. The goal is to validate real-world usefulness rather than add speculative functionality.
Pom is also a Rust learning project, so development progresses incrementally.

------

## Why this exists

Because:

- Writing the same CMake boilerplate and directory structure for every embedded project gets old
- Good structure at day 0 prevents pain at month 6
- Bash scripts I used for this previously are not easily scalable

-----

## License

Pom is licensed under the MIT License. See `LICENSE` for details.

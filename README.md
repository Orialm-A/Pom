# Pom

![License](https://img.shields.io/badge/license-MIT-blue.svg) [![CI](https://github.com/Orialm-A/Pom/actions/workflows/ci.yml/badge.svg)](https://github.com/Orialm-A/Pom/blob/release/.github/workflows/ci.yml) ![Status](https://img.shields.io/badge/status-alpha-red)

Pom is a small CLI tool to scaffold and organize embedded C projects.

It focuses on structure, consistency and long-term maintainability rather than flashy automation.

> Like Polyoxymethylene, Pom is used where precision, rigidity and low friction matter.

---
## ⚠️ Project Status
Pom is currently in **alpha**:
- Only a limited feature set is implemented
- No installation or distribution mechanism exists yet
- The project is not intended for external usage at this stage

This repository mainly documents ongoing development.

---
## Features

- [x] Create projects
- [x] Add modules
- [x] Rename modules
- [ ] Generate CMake files
- [ ] Provide target-independent `build` and `flash` commands
- [ ] Generate RTOS-ready projects
- [ ] Additional targets
- [ ] Add / remove directories
- [ ] User configuration

---
## How to use it
### Installation
*Pom cannot be installed yet. A clean Pom installation process is planned after RTOS selection is implemented. No date is planned yet.*

### Current CLI Overview
```bash
pom project create [NAME] [--path PATH] [--target TARGET]
pom module add [NAME] [--layer LAYER] [--brief TEXT] [--details TEXT]
pom module rename [OLD_NAME] [NEW_NAME] [--yes]
```

Other commands (`build`, `flash`, `monitor`, `clean`, etc.) exist in the CLI structure but are not implemented yet.

## License

Pom is licensed under the MIT License. See `LICENSE` for details.

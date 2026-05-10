## Current CLI Overview

```bash
pom project create [NAME] [--path PATH] [--target TARGET]
pom module add [NAME] [--layer LAYER] [--brief TEXT] [--details TEXT]
pom module rename [OLD_NAME] [NEW_NAME] [--yes]
```

Other commands (`build`, `flash`, `monitor`, `clean`, etc.) exist in the CLI structure but are not implemented yet.

## `pom project create` 

This command is used to create a project:

```sh
pom project create [NAME] [--path PATH] [--target TARGET]
```

### Parameters

- **`[NAME]`** ***- optional, positional argument - project name***:
  Pom will automatically derive 2 strings from this parameter: 

  - A normal one, as typed by the user, used in human-facing documents (like the `README.md`)
  - A slugified version, in `snake_case`, without whitespaces or special characters (for directory creation)

  If no name is specified, the user is prompted for one.

- **[--path]** ***- optional, named argument - project location***:
  If none is set, Pom defaults to the current working directory.

- **[--target]** ***- optional, named argument - project target***:
  If the typed target is not found or if none is provided, the user is prompted to select one of the available targets in a menu.
  Few default targets are embedded in Pom. Additional default targets might be added later. It is planned to let users define their own targets in future versions.

### Result

Pom will create a directory with the specified name and location, considered as the project's root. In this directory, it will create directories described in `default_generation_layer.toml`. It will copy files (like `.gitignore`) from a default location in the root of the project. It is planned to let users define their own generation layout and files in future versions, overriding default directories and files.

Pom also generates in project root:

- `pom.toml` - Describes in which directories C modules can be created by pom, with what prefix for the files name
-  `doc_groups.h` - Describes Doxygen groups for modules

## `pom module add`

This command is used to add a C module to a project:

```sh
pom module add [NAME] [--layer LAYER] [--brief TEXT] [--details TEXT] [--no-prefix]
```

### Parameters

- **`[NAME]`** ***- optional, positional argument - module name***:
  Pom will automatically derive 2 strings from this parameter: 

  - A slugified version, in `snake_case`, without whitespaces or special characters (for files name)
  - A second slugified version, in `CONST_CASE` (for include guard)

  If no name is specified, the user is prompted for one. If the name is already used for another module in the project, Pom will warn the user and prompt for confirmation.

- **[--layer]** ***- optional, named argument - in what layer the module must be created***:
  A layer represents a logical level in the firmware: `app`, `devices`... This definition is extended to any directory in which Pom is allowed to create modules, listed in `pom.toml`.
  If the specified layer is not found or if none is provided, the user is prompted to select one of the available layers in a menu.

- **[--brief]** ***- optional, named argument - module's brief for Doxygen***:
  if none is provided, the user is prompted to type one. They can pass the prompt without typing anything by pressing Enter.

- **[--details]** ***- optional, named argument - module's details for Doxygen***:
  if none is provided, the user is prompted to type one. They can pass the prompt without typing anything by pressing Enter.

- **[--no-prefix]** ***- Optional flag - Create a module without a layer's prefix in file name***

### Result

Pom will create `*.h` and `*.c` files with the specified name, in the specified layer. Unless the flag `--no-prefix` is used, the files name will start with a prefix associated the layer. For instance, `a_` if the module is located in `app/`. The created files contain a header, and sections, according to a default template. It is planned to let user define their own template files in future versions, overriding default templates.

## `pom module rename`

This command is used to rename an existing C module:

```sh
pom module rename [OLD_NAME] [NEW_NAME] [--yes]
```

### Parameters

- **`[OLD_NAME]`** ***- optional, positional argument - old module name***:
  If no name is specified, the user is prompted for one. Pom will automatically search for the named module in the project:
  - If it cannot find any module with this name, an error occurs
  - If it finds a single module with this name, it is automatically selected
  - If it finds several module with this name, the user is prompted to select one from a menu showing the available locations.

- **`[NEW_NAMENAME]`** ***- optional, positional argument - new module name***:
  Pom will automatically derive 2 strings from this parameter: 

  - A slugified version, in `snake_case`, without whitespaces or special characters (for files name)
  - A second slugified version, in `CONST_CASE` (for include guard)

  If no name is specified, the user is prompted for one. If the name is already used for another module in the project, Pom will warn the user and prompt for confirmation.

- **[--yes]** ***- optional flag - proceed to rename without confirmation prompt***

### Result

Pom will display the module location, the old name, and the new name. Unless the flag `--yes` is used, the user will be prompted to confirm the rename. After the rename has completed, Pom will iterate over the project files in the directories where it is allowed to create module. it will look for the line including the renamed module. These includes might be updated if one of these conditions is met, tested in this order:

1) ***The old module name was unique***:
   If there was only one module using the old name, there is no ambiguity on the included header.
2) ***The include path is fully resolved***:
   Pom can create module in several directories. If the include path starts in the root of these authorized directories, it is considered fully resolved: There's no truncated part *before* the specified path. Given a path to a file is unique, there's no ambiguity on the included header. "The root of these authorized directories" is defined as the set of first element of all the directories path where Pom can create modules. For example, for `src/app`, `src/devices` and `unit_tests/motor`, it is `src` and `unit_tests`.
3) ***The include path can be resolved from the file containing it***:
   Pom compares the path to the renamed module (relative to project's root) *with the old name* and the path obtained by combining the path to the current file (relative to project's root) and the include path. If they match, a path being unique, there's no ambiguity on the included header.
4) ***The user validate the update***:
   If Pom cannot remove an ambiguity with the 3 criteria above, it prompts the user for confirmation.
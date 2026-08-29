# When to add a manifest

Add one manifest for each board profile. The profile must describe facts from
the board and MCU documentation, not values guessed from another board.

Adding a manifest alone does not add kernel support. An accepted board also
requires a reviewed `crates/dali-boards/<board-crate>/` backend, target checks, documentation,
and hardware evidence. Do not copy values into CLI commands or ordinary
implementation modules.

## File location and naming

Manifests live directly under the repository `targets/` directory:

```text
targets/<profile-name>.toml
```

The file name is organizational. The stable profile name is the value of
`[profile].name` and is used by CLI commands and application configuration.
Profile names must be unique and should use lowercase kebab-case or lowercase
alphanumeric names consistent with existing commands.

## Sections

Every manifest contains:

- `[profile]` — identity and Dali compatibility;
- `[artifacts]` — optional conventional build artifact names;
- `[profile.dfu]` — optional USB DFU identity and download configuration;
- `[capabilities]` — explicit backend and runtime capability declarations;
- `[clock]` — oscillator and bus frequencies;
- `[i2c]` — I2C bus timing configuration;
- `[display]` — optional I2C OLED controller and geometry configuration;
- `[driver_probe]` — bounded hardware acceptance probe configuration;
- `[memory]` — kernel, application, and runtime regions;
- `[status_led]` — logical status LED mapping;
- `[user_key]` — board user-key mapping;
- `[usb]` — USB FS controller and data pins.

`[storage]` is optional because a board profile may be described before its
storage transport is supported. Storage must not be omitted from a profile
that claims a kernel storage backend.

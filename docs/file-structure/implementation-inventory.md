# Exact implementation file inventory

The directory tree above groups related files. The current tracked source
files inside those groups are:

```text
kernel/src/
├── lib.rs
├── main.rs
├── abi.rs
├── security/{mod.rs,
│   fault/{mod.rs,persistent.rs,scb.rs},launch/{mod.rs},
│   mpu/{mod.rs,descriptor.rs,layout.rs,hardware.rs,tests.rs},
│   privilege/{mod.rs,svc.rs},scheduling/{mod.rs}}
├── platform/{mod.rs}
├── bootstrap/{mod.rs,
│   startup/{mod.rs,logging.rs,watchdog.rs},
│   lifecycle/{mod.rs,heartbeat.rs,status.rs},
│   storage/{mod.rs,initialization.rs,acceptance.rs},
│   loading/{mod.rs,cartridge.rs}}
├── drivers/{mod.rs,block.rs,sdio.rs}
├── loader/{mod.rs,repository/{mod.rs,amrn.rs,chain/{mod.rs,bundle.rs,loading.rs,roles.rs,streaming.rs,types.rs,validation.rs},discovery.rs,installation.rs,io.rs,streaming.rs,trust.rs,tests.rs},contract/{mod.rs,catalog.rs,tests.rs},pipeline/{mod.rs,execution.rs,relocation.rs,identity.rs,discovery.rs,services.rs,signed.rs}}
├── logging/{mod.rs,rtt.rs,usb_cdc.rs}
├── runtime/{mod.rs,application/{mod.rs,lifecycle.rs,owner.rs,policy.rs},memory/{mod.rs,dma.rs,slots.rs},scheduling/{mod.rs,context_switch.rs,saved_state.rs,record.rs,context_table.rs,scheduler.rs,storage.rs,tick.rs},watchdog/mod.rs}
└── storage/{mod.rs,durable.rs,durable/{journal.rs,coordinator.rs},filesystem/{mod.rs,artifacts.rs,multi.rs,read.rs,tests.rs,write.rs},repository.rs}

crates/dali-boards/
├── src/lib.rs
└── dali-board-stm32f405/
    ├── Cargo.toml
    └── src/{lib.rs,architecture.rs,backend.rs,board.rs,exceptions.rs,logging.rs,
        mpu.rs,scheduling.rs,sdio.rs,watchdog.rs,
        board/{acceptance.rs,config.rs,initialization.rs,input.rs,resources.rs,
        scheduler.rs,services.rs,usb.rs},
        drivers/{mod.rs,gpio.rs,i2c.rs,interrupt.rs,probe.rs,spi.rs,ssd1306.rs,
        timeout.rs,timer.rs,uart.rs},
        sdio_raw/{mod.rs,dma.rs,init.rs,status.rs,write.rs},
        mpu/{descriptor.rs,hardware.rs,layout.rs}}

crates/dali-amrn/src/
├── lib.rs                         # Stable crate facade and legacy re-exports
├── compatibility/mod.rs           # ABI-to-format compatibility rules
├── legacy/                        # Original fixed-origin cartridge contract
│   ├── mod.rs, builder.rs, stream.rs, tests.rs
├── v2/                            # Segmented ABI v3 cartridge contract
│   ├── mod.rs, tests.rs
├── v3/                            # Relocatable ABI v3 cartridge contract
│   ├── mod.rs, apply.rs, codec.rs, wire.rs, tests.rs
├── v4/                            # Identity and selection metadata extension
│   ├── mod.rs, tests.rs
└── v5/                            # Signed container extension; kernel-gated
    ├── mod.rs, codec.rs, tests.rs

crates/dali-cli/src/
├── main.rs
└── commands/
    ├── mod.rs, doctor.rs, inspect.rs, key.rs, cartridge.rs
    ├── app/
    │   ├── mod.rs, artifacts.rs, build.rs, init.rs, linker.rs
    │   ├── new.rs, new_tests.rs, cartridge.rs, relocations.rs
    ├── device/
    │   ├── mod.rs, attach.rs, cdc.rs, console.rs, flash.rs
    │   ├── flash_transport.rs, info.rs
    ├── metadata/
    │   ├── mod.rs
    │   ├── delegation/mod.rs
    │   ├── repository/{mod.rs,common.rs,init.rs,add_developer.rs,
    │   │   manifest.rs,publish.rs,register_cartridge.rs}
    │   └── bundle/{mod.rs,common.rs,generate.rs,verify.rs}
    └── target/{mod.rs,info.rs}

crates/dali-cli/templates/app/
├── Cargo.toml.template, build.rs.template, config.toml.template
├── dali.toml.template, lib.rs.template, main.rs.template
└── memory.x.template, memory.v3.x.template

crates/dali-device/src/lib.rs
crates/dali-driver-api/src/
├── lib.rs
├── error/mod.rs
├── display/{mod.rs,console.rs,dimensions.rs,driver.rs,text.rs}
├── gpio/{mod.rs,interrupt.rs,pins.rs}
├── i2c/{mod.rs,address.rs,driver.rs}
├── uart/{mod.rs,config.rs,read.rs,write.rs,ownership.rs}
├── spi/{mod.rs,ownership.rs,transfer.rs}
└── timer/{mod.rs,countdown.rs,driver.rs}
crates/dali-metadata/src/
├── lib.rs, authorization.rs, chain.rs, crypto.rs, limits.rs, model.rs,
│   streaming.rs, trust_store.rs, validation.rs, verification.rs
├── codec/{mod.rs,binary/mod.rs,bundle.rs,delegation.rs,envelope.rs,
│   revocation.rs,root.rs,signatures.rs,snapshot.rs,targets.rs,timestamp.rs,
│   writer.rs,tests.rs}
├── model/{bundle.rs,core.rs,delegation.rs,repository.rs,revocation.rs,
│   snapshot.rs,targets.rs,trust_store.rs}
├── parser/
│   ├── mod.rs,tests.rs
│   ├── binary/{mod.rs,bundle.rs,delegation.rs,helpers.rs,revocation.rs,
│   │   root.rs,snapshot.rs,targets.rs,tests.rs,trust_store.rs}
│   ├── canonical/
│   │   ├── mod.rs,cursor.rs,bundle.rs,envelope.rs,signatures.rs,
│   │   └── delegation.rs,revocation.rs,root.rs,snapshot.rs,targets.rs,
│   │       timestamp.rs
│   └── streaming/
│       ├── mod.rs,chain.rs,delegation.rs,revocation.rs,root.rs,snapshot.rs
│       └── bundle/{mod.rs,parser.rs,queue.rs,tests.rs}
├── secure_boot/{mod.rs,descriptor.rs,tests.rs}
└── validation/{bundle.rs,common.rs,core.rs,delegation.rs,records.rs,
    tests.rs,trust_store.rs}
crates/dali-sdk/src/{lib.rs,svc.rs,svc_log.rs}
crates/dali-targets/src/lib.rs
crates/dali-targets/build.rs
crates/dali-targets/build/{loader.rs,manifest.rs,render.rs,
│   generator/{mod.rs,authentication.rs,hardware.rs,profile.rs},
│   validation/{mod.rs,memory.rs,security.rs}}
crates/dali-usb/src/{lib.rs,tests.rs}
```

Each application fixture has its own `Cargo.toml`, `build.rs`, `src/lib.rs`,
and `src/main.rs`. The fault and SVC fixtures additionally have a local
`.cargo/config.toml`, `Cargo.lock`, and `dali.toml`. The relocation fixture has
its own `Cargo.lock` but uses the root build configuration. `dali-app-hello`
The kernel build script generates its linker `memory.x` from the selected
target manifest; the generated script is not tracked.

The CLI documentation files currently tracked under `docs/cli/commands/` are:

```text
app-build.md, app-init.md, app-new.md, app-cartridge.md,
device-attach.md, device-console.md, device-flash.md, device-info.md,
doctor.md, inspect.md, cartridge.md, target-info.md, target-list.md,
target-scaffold.md
```

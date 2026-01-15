# kernel-rust-slab-Fahed-MASAD

Projet d'examen Rust **no_std** / **no_main** : un micro-kernel éducatif x86_64 avec logs série, IDT, heap fixe, bump allocator et slab allocator minimal.

> ⚠️ Toute reprise de code est créditée en section [Credits](#credits).

## Objectifs couverts

1. Boot QEMU + logs série.
2. UART COM1 (0x3F8) + macros `printk!` / `serial_println!`.
3. `panic_handler` qui log sur série.
4. Exceptions (breakpoint, double fault, page fault) + dump minimal des registres.
5. Memory map documentée + heap fixe.
6. Bump allocator minimal.
7. Slab allocator minimal avec free list O(1).
8. Tests host (std) + smoke test QEMU.
9. Write-up SLUB dans `docs/slub_writeup.md`.
10. Instructions `fmt` / `clippy`.

## Arborescence

```
.
├── .cargo/config.toml
├── x86_64-kernel.json
├── rust-toolchain.toml
├── Cargo.toml
├── README.md
├── Authors.md
├── docs/
│   └── slub_writeup.md
├── scripts/
│   ├── run_qemu.sh
│   └── test_qemu.sh
└── src/
    ├── main.rs
    ├── lib.rs
    ├── serial.rs
    ├── gdt.rs
    ├── interrupts.rs
    ├── memory.rs
    └── alloc/
        ├── mod.rs
        ├── bump.rs
        ├── locked.rs
        └── slab.rs
```

## Choix techniques

- **Bootloader** : crate `bootloader`/`bootloader_api` (solution standard OSDev Rust).
- **Arch** : x86_64.
- **Crates OSDev** : `x86_64`, `uart_16550`, `spin`, `lazy_static`, `pic8259`.

## Memory map (résumé)

- Le bootloader fournit la mémoire physique via `BootInfo`.
- On mappe un heap virtuel fixe :
  - `HEAP_START = 0x4444_4444_0000`
  - `HEAP_SIZE  = 1 MiB`
- Ce heap est ensuite initialisé par le slab allocator.

## Commandes à lancer

### 0) Pré-requis

```bash
rustup toolchain install nightly
rustup component add rust-src llvm-tools-preview
cargo install bootimage
```

### 1) Build du kernel

```bash
cargo build
```

### 2) Lancer QEMU (boot + logs série)

```bash
./scripts/run_qemu.sh
```

### 3) Tests host (std) pour le slab allocator

```bash
cargo test --lib
```

### 4) Smoke test QEMU (test runner noyau)

```bash
./scripts/test_qemu.sh
```

### 5) Format / Clippy

```bash
cargo fmt
cargo clippy --all-targets --all-features
```

## Stratégie de commits (granularité recommandée)

1. **chore: bootstrap kernel skeleton** (no_std/no_main, bootloader, config cible, entry point).
2. **feat: serial logger + printk macros** (UART, panic logs).
3. **feat: gdt/idt + exceptions** (breakpoint/double fault/page fault).
4. **feat: memory map + heap mapping** (frame allocator + heap fixe).
5. **feat: bump allocator**.
6. **feat: slab allocator minimal** (free list O(1), caches).
7. **test: host tests slab** (stress/reuse/align).
8. **test: qemu smoke test**.
9. **docs: README + slub writeup + Authors**.
10. **chore: scripts qemu + bundle instructions**.

## Procédure Git bundle

```bash
git bundle create kernel-rust-slab.bundle --all
```

## Troubleshooting

- **QEMU ne boot pas** : vérifier l'installation de `bootimage` et la toolchain nightly.
- **Pas de sortie série** : vérifier que QEMU est lancé avec `-serial stdio` (voir scripts).
- **Triple fault** : vérifier GDT/TSS/IDT et le stack de double fault.

## Credits

- Phil Opp, *Writing an OS in Rust* (structure du boot, IDT, heap mapping, tests QEMU).
- Crates: `bootloader`, `x86_64`, `uart_16550`, `spin`, `lazy_static`, `pic8259`.

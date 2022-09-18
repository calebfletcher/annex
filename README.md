# Annex
Annex is an experimental kernel developed for RISC-V in pure Rust (and a sprinkling of assembly). It's primary hardware target is the StarFive VisionFive 2, and has also been developed against QEMU.

## Prerequisites

### Toolchain
This project requires several dependencies to run. Most importantly, you require a RISC-V toolchain. This project has been developed against SiFive's Freedom toolchain, available on their GitHub, https://github.com/sifive/freedom-tools.

### QEMU
Additionally, to emulate the kernel without running on real hardware, QEMU is required. Apt's version of QEMU is quite old, so it's better to build it yourself, see https://wiki.qemu.org/Hosts/Linux.

### U-Boot
Finally, if you want to use U-Boot for running the kernel in QEMU, you will also need to clone and build a U-Boot binary. This requires U-Boot to be cross-compiled for RISC-V, so make sure you have the toolchain installed first.

```
CROSS_COMPILE=riscv64-linux-gnu- ARCH=riscv make qemu-riscv64_smode_defconfig
CROSS_COMPILE=riscv64-linux-gnu- ARCH=riscv make -j12 tools u-boot.bin
```

## Usage
To run the kernel directly under OpenSBI in QEMU, run `just qemu-raw`. To run it under U-Boot, run `just qemu-uboot`.

To run the kernel on real hardware (presumably through U-Boot), run `just uimage` and either copy the resulting `kernel.uimage` to an SD card, or burn the `kernel.img` to the SD card directly.

You can see the full list of available commands through `just -l`.

## U-Boot Configuration
In order to run the kernel through U-Boot, a uImage is provided. This uImage can be copied onto a USB or an SD card and then loaded and executed by U-Boot.

While these steps will be slightly hardware-specific, these are the steps to load it when using QEMU. This will load the `kernel.uimage` file from the virtio driver, and then boots it, while providing the address to the FDT. In standard Linux(?) form, this will execute the entrypoint so that the register `a0` will contain the hart id, and `a1` will contain the FDT address.

1. `fatload virtio 0 0x84000000 kernel.uimage`
2. `bootm 0x84000000 - ${fdtcontroladdr}`

This can be configured in U-Boot as the default command, by setting `CONFIG_BOOTCOMMAND` to be these two steps.
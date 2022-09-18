
# Usage
To run the kernel directly under OpenSBI in QEMU, run `just qemu-raw`. To run it under U-Boot, run `just qemu-uboot`.


# U-Boot Configuration
In order to run the kernel through U-Boot, a uImage is provided. This uImage can be copied onto a USB or an SD card and then loaded and executed by U-Boot.

While these steps will be slightly hardware-specific, these are the steps to load it when using QEMU. This will load the `kernel.uimage` file from the virtio driver, and then boots it, while providing the address to the FDT. In standard Linux(?) form, this will execute the entrypoint so that the register `a0` will contain the hart id, and `a1` will contain the FDT address.

1. `fatload virtio 0 0x84000000 kernel.uimage`
2. `bootm 0x84000000 - ${fdtcontroladdr}`

This can be configured in U-Boot as the default command, by setting `CONFIG_BOOTCOMMAND` to be these two steps. Ensure that the 
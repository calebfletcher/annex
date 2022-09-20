# Notes on the RISC-V ISA and Common Implementations

As the RISC-V ecosystem is fairly new, there is a lack of available documentation and discussions online regarding some of the more technical details, especially those that are specific to individual implementations of the the ISA. This document aims to collect some of these details that have been found through searching through documentation or other sources online, that are sometimes hard to find.

## CLINT Timer
The CLINT timer register `mtime`, also called the timebase register, is defined by the privileged ISA to increment at a constant frequency, but that that frequency is implementation-defined and should be told to the software. It seems as though this frequency is actually passed through a dedicated node in the device tree, `/cpus/timebase-frequency`. This is not made clear in either RISC-V or SiFive's specifications though, and should be confirmed for your CPU. It appears that this is comes from a PowerPC timebase node originally, and has been reused.

Where QEMU implements this node:
https://github.com/qemu/qemu/blob/d29201ff34a135cdfc197f4413c1c5047e4f58bb/hw/riscv/virt.c#L740
https://github.com/qemu/qemu/blob/d29201ff34a135cdfc197f4413c1c5047e4f58bb/include/hw/intc/riscv_aclint.h#L78

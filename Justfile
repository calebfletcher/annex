# Kernel Parameters
load_addr := "0x80200000"
entrypoint_addr := "0x80200000"
uimage_name := "annex"
image_size_mb := "10"
profile := "release"

# Emulation
qemu_cmd := "qemu-system-riscv64"
qemu_machine := "-M virt -smp 4 -nographic -m 1G -device qemu-xhci -device usb-kbd"

# System Setup
compiler_prefix := "riscv64-unknown-elf-"
out_dir := "target/riscv64gc-unknown-none-elf/" + profile + "/"
u-boot_dir := "../../u-boot/"
mount_dir := "./img_mount"


# File Paths
u-boot_path := u-boot_dir + "u-boot.bin"
lib_file := out_dir + "libannex_risc_v.a"
elf_file := out_dir + "annex-risc-v"
bin_file := out_dir + "kernel.bin"
uimage_file := out_dir + "kernel.uimage"
img_file := out_dir + "kernel.img"

# Clean the build artifacts
clean:
    cargo clean
    rm -rf {{mount_dir}}

# Build the kernel ELF
kernel:
    cargo b --profile {{ if profile == "debug" { "dev" } else { "release" } }}

# Convert the ELF file to a raw binary executable
binary: kernel
    {{compiler_prefix}}objcopy {{elf_file}} -O binary {{bin_file}}

# Initialise an image file with a FAT partition
image-init:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ ! -f {{img_file}} ]; then
        dd if=/dev/zero of={{img_file}} count={{image_size_mb}} bs=1M status=none
        sudo parted {{img_file}} mklabel msdos mkpart primary 2048s 100%
        mkfs.fat {{img_file}}
        mkdir -p {{mount_dir}}
    fi

# Mount the image and copy a file into it
image file: image-init
    sudo mount {{img_file}} {{mount_dir}}
    sudo cp {{file}} {{mount_dir}}/
    sudo umount {{mount_dir}}

# Create a raw image
raw-image: binary && (image bin_file)

# Create a uImage-based image
uimage: binary && (image uimage_file)
    {{u-boot_dir}}tools/mkimage -A riscv -O linux -T kernel -C none -a {{load_addr}} -e {{entrypoint_addr}} -n {{uimage_name}} -d {{bin_file}} {{uimage_file}}

# Emulate the kernel with the raw ELF kernel
qemu-raw: kernel
    {{qemu_cmd}} {{qemu_machine}} -s -kernel {{elf_file}}

# Emulate the kernel through U-Boot
qemu-uboot: uimage
    {{qemu_cmd}} {{qemu_machine}} -s -kernel {{u-boot_path}} -device virtio-blk-device,drive=hd0 -drive if=none,format=raw,id=hd0,file={{img_file}}

# Open GDB on the kernel
gdb:
    {{compiler_prefix}}gdb {{elf_file}} -ex "target remote :1234" -ex "b pre_main"
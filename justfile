project_dir := justfile_directory()

build := "release"
mode := "bios"
cargo_release_flag := if build == "release" { "--release" } else { "" }
mode_flags_qemu := if mode == "uefi" { "-bios " + project_dir + "/OVMF-pure-efi.fd" } else { "" }

bootloader_dir := parent_directory(`cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "bootloader") | .manifest_path'`)
target_dir := join(justfile_directory(), "target")
kernel_binary := join(target_dir, "x86_64-annex", build, "annex")
out_dir := parent_directory(kernel_binary)
disk_image := out_dir + "/boot-" + mode + "-" + file_name(kernel_binary) + ".img"

is_test := if file_name(parent_directory(kernel_binary)) == "deps" { "true" } else { "false" }

#qemu_bin := "/opt/qemu-7.0.0/build/qemu-system-x86_64"
qemu_bin := "qemu-system-x86_64"
qemu_args := "-machine q35,accel=kvm --enable-kvm -cpu host -m 256M -vga std -device isa-debug-exit,iobase=0xf4,iosize=0x04 -drive format=raw,file=" + disk_image + " -serial stdio -no-reboot -no-shutdown -s " + mode_flags_qemu

_default:
    just --list

build-kernel:
    cargo b {{cargo_release_flag}}

build-image:
    cd {{bootloader_dir}} && \
        cargo builder   --kernel-manifest {{project_dir}}/Cargo.toml \
                        --kernel-binary {{kernel_binary}} \
                        --target-dir {{target_dir}} \
                        --out-dir {{out_dir}} \
                        --quiet

build: build-kernel build-image

qemu-test:
    #!/usr/bin/env bash
    set -uxo pipefail

    {{qemu_bin}} \
        {{qemu_args}} \
        -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
        -display none; (( $?==33 ))

qemu:
    #!/usr/bin/env bash
    set -euxo pipefail

    if {{is_test}}; then
        timeout --foreground 60s just kernel_binary={{kernel_binary}} qemu-test
    else
        {{qemu_bin}} {{qemu_args}}
    fi    

# Run QEMU but wait for debugger
qemu-dbg: build
    {{qemu_bin}} {{qemu_args}} -S

gdb:
    rust-gdb {{kernel_binary}} -ex "target remote :1234" -ex "b entry_point" -ex "c"

runner binary:
    just kernel_binary={{absolute_path(binary)}} build-image
    just kernel_binary={{absolute_path(binary)}} qemu

clean:
    cargo clean
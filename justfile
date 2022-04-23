build := "release"
release_flag := if build == "release" { "--release" } else { "" }

bootloader_dir := parent_directory(`cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "bootloader") | .manifest_path'`)
disk_image := bootloader_dir + "/target/x86_64-bootloader/release/boot-bios-annex.img"
target_dir := justfile_directory() + "/target"
kernel_binary := target_dir + "/x86_64-annex/" + build + "/annex"
out_dir := parent_directory(kernel_binary)

is_test := if file_name(parent_directory(kernel_binary)) == "deps" { "true" } else { "false" }

_default:
    just --list

build-kernel:
    cargo b {{release_flag}}

build-image:
    cd {{bootloader_dir}} && \
        cargo builder   --kernel-manifest {{justfile_directory()}}/Cargo.toml \
                        --kernel-binary {{kernel_binary}} \
                        --target-dir {{target_dir}} \
                        --out-dir {{out_dir}} \
                        --quiet

build: build-kernel build-image

qemu-test:
    #!/usr/bin/env bash
    set -uxo pipefail

    qemu-system-x86_64 \
        -drive format=raw,file={{out_dir}}/boot-bios-{{file_name(kernel_binary)}}.img \
        -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
        -serial stdio -display none; (( $?==33 ))

qemu:
    #!/usr/bin/env bash
    set -euxo pipefail

    if {{is_test}}; then
        timeout --foreground 60s just kernel_binary={{kernel_binary}} qemu-test
    else
        qemu-system-x86_64 -drive format=raw,file={{out_dir}}/boot-bios-{{file_name(kernel_binary)}}.img -serial stdio
    fi    

runner binary:
    just kernel_binary={{absolute_path(binary)}} build-image
    just kernel_binary={{absolute_path(binary)}} qemu

clean:
    cargo clean
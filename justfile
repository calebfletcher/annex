build := "release"
release_flag := if build == "release" { "--release" } else { "" }

bootloader_dir := parent_directory(`cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "bootloader") | .manifest_path'`)
disk_image := bootloader_dir + "/target/x86_64-bootloader/release/boot-bios-annex.img"
target_dir := justfile_directory() + "/target"
out_dir := target_dir + "/x86_64-annex/" + build

_default:
    just --list

build-kernel:
    cargo b {{release_flag}}

build-image:
    #!/usr/bin/bash
    cd {{bootloader_dir}}
    cargo builder   --kernel-manifest {{justfile_directory()}}/Cargo.toml \
                    --kernel-binary {{out_dir}}/annex \
                    --target-dir {{target_dir}} \
                    --out-dir {{out_dir}}

build: build-kernel build-image

qemu: build
    qemu-system-x86_64 -drive format=raw,file={{out_dir}}/boot-bios-annex.img

clean:
    cargo clean
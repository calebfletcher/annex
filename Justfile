qemu_cmd := "qemu-system-riscv64"
qemu_machine := "-M virt -smp 4 -nographic -m 1G"
u-boot_path := "../../u-boot/u-boot.bin"
out_dir := "target/riscv64gc-unknown-none-elf/debug/"
lib_file := out_dir + "libannex_risc_v.a"
elf_file := out_dir + "kernel.elf"
bin_file := out_dir + "boot.bin"
img_file := "./kernel.img"
mount_dir := "./img_mount"

clean:
    cargo clean
    rm -rf mount_dir

image-init:
    dd if=/dev/zero of={{img_file}} count=50 bs=1M
    sudo parted -a optimal {{img_file}} mklabel msdos mkpart primary 0G 100%
    mkfs.fat {{img_file}}
    mkdir -p {{mount_dir}}

kernel:
    cargo b
    riscv64-unknown-elf-gcc -T src/lds/virt.lds -o {{elf_file}} -nostdlib {{lib_file}}

image: kernel
    riscv64-unknown-elf-objcopy {{elf_file}} -O binary {{bin_file}}
    sudo mount {{img_file}} {{mount_dir}}
    sudo cp {{bin_file}} {{mount_dir}}/
    sudo umount {{mount_dir}}

qemu-raw: kernel
    {{qemu_cmd}} {{qemu_machine}} -s -bios {{elf_file}}

qemu: image
    {{qemu_cmd}} {{qemu_machine}} -s -kernel {{u-boot_path}} -device virtio-blk-device,drive=hd0 -drive if=none,format=raw,id=hd0,file={{img_file}}

gdb:
    riscv64-unknown-elf-gdb {{elf_file}} -ex "target remote :1234" -ex "b pre_main"
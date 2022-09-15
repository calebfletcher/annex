qemu_cmd := "qemu-system-riscv64"
qemu_machine := "-M virt -smp 4 -nographic -m 1G"
u-boot_path := "../../u-boot/u-boot.bin"

kernel:
    cargo b
    riscv64-unknown-elf-gcc -T src/lds/virt.lds -o out.elf -ffreestanding -O0 -nostdlib target/riscv64gc-unknown-none-elf/debug/libannex_risc_v.rlib

image: kernel
    riscv64-unknown-elf-objcopy out.elf -O binary boot.bin
    dd if=/dev/zero of=test.img count=50 bs=1M
    sudo parted -a optimal ./test.img mklabel msdos mkpart primary 0G 100%
    mkfs.fat test.img
    mkdir -p img_mount
    sudo mount ./test.img ./img_mount
    sudo cp boot.bin ./img_mount/
    sudo umount ./img_mount

qemu-raw: kernel
    {{qemu_cmd}} {{qemu_machine}} -s -bios out.elf

# qemu-uboot: kernel
#     {{qemu_cmd}} {{qemu_machine}} -s -kernel {{u-boot_path}} -device virtio-blk-device,drive=hd0 -drive if=none,format=raw,id=hd0,file=./out.elf

qemu: image
    {{qemu_cmd}} {{qemu_machine}} -s -kernel {{u-boot_path}} -device virtio-blk-device,drive=hd0 -drive if=none,format=raw,id=hd0,file=./test.img

gdb:
    riscv64-unknown-elf-gdb out.elf -ex "target remote :1234" -ex "b pre_main"
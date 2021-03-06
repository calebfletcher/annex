# annex
Hobby kernel to experiment with OS development.


# Installation Tips
Don't use APT's version of QEMU, it is way too old. Build it from source instead. By default QEMU installs every target, if this is not what you want (since it takes an awfully long time) then configure it to only install x86_64.

The UEFI firmware for QEMU is 'OVMF-pure-efi.fd' from here: https://github.com/rust-osdev/ovmf-prebuilt/releases/tag/v0.20211216.165%2Bg96e1d337e0.

# Hardware
```
sudo dd if=target/x86_64-annex/release/boot-uefi-annex.img of=/dev/sdX
```

# Debugging Tips and Tricks
Get current thread:
```
print annex::threading::scheduler::SCHEDULER.inner.data.value.0.current_thread_id.0
print annex::threading::scheduler::SCHEDULER.inner.data.value.0.threads.root.0.node.pointer.vals[0].value.value.state
```
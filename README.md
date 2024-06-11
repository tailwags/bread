# Bread

An experimental UEFI bootloader for the Linux kernel written in Rust.

Unlike other UEFI-only bootloaders, Bread does not use the kernel UEFI-stub.
Instead, it relies on the same boot protocol that has been used in traditional
BIOS booting for years. This allows for greater control over the memory map and
enables more advanced boot scenarios, such as secure boot and encryption,
without requiring special support from the kernel.

Currently, the following architectures are supported:

- x86-64
- aarch64

In the future, riscv64gc will also be a target, but currently, there are no
actual UEFI riscv boards available.

## Building

Before building Bread, there are a few prerequisites:

- [just](https://just.system) must be installed and available in the PATH.
- Rust must be installed via [rustup](https://rustup.rs/). Distro-provided rust
  versions will not work.
- [QEMU](https://www.qemu.org/) can be optionally installed to run the
  bootloader in a VM.

To build Bread, simply run `just` in the terminal. You can then copy the
resulting `bread.efi` file to your EFI partition.

To run inside a vm, ensure QEMU is available in the PATH, then run:

```
just qemu-x86_64 # or qemu-aarch64
```

This will create a new ESP folder containing the required files and launch QEMU
using the scripts located in the `scripts` folder.

## Configuring

TODO

## Effie

To interact with the UEFI system, Bread uses a custom library called `effie`,
which can be found in the corresponding folder. While effie might seem like a
general-purpose UEFI library, its scope is much narrower and is tailored to the
specific needs of this project. For this reason, most protocols are not
implemented unless absolutely required. Nevertheless, this library can be used
standalone to write UEFI programs. If you need a protocol to be implemented,
feel free to open a PR.

## License

This project is licensed under the
[APACHE-2.0 LICENSE](http://www.apache.org/licenses/LICENSE-2.0). You can find
more info in the [LICENSE](LICENSE) file.

# Configuring Bread

Bread reads its config from `\EFI\bread\bread.conf`, on the same ESP it was
loaded from. On NixOS this file is written by the installer on every rebuild,
so any manual changes will be overwritten.

## Syntax

The format is line-based:

- Every line is a **key**, a space or tab, then a **value** that runs to the
  end of the line. Values are never quoted or escaped.
- Indentation is purely cosmetic. Both LF and CRLF line endings work.
- Blank lines and lines starting with `#` are comments. A `#` anywhere else is
  part of the value.
- `linux <id>`, `efi <id>` and `folder <id>` open an entry, and `end` closes
  it. Folders can contain other entries, including other folders.

## Keys

At the top level:

| Key       | Value                                         | Default                |
| --------- | --------------------------------------------- | ---------------------- |
| `timeout` | Seconds before the default entry boots        | Wait forever           |
| `default` | Id of the entry selected at startup           | The first entry        |
| `font`    | A file reference to a PSF2 font for the menu  | The built-in font      |

Every entry needs a `name`, the label shown in the menu. The other keys depend
on the type of entry:

| Entry    | Key       | Value                                                  | Required |
| -------- | --------- | ------------------------------------------------------ | -------- |
| `linux`  | `kernel`  | A file reference to the kernel image                   | Yes      |
| `linux`  | `initrd`  | A file reference to an initrd. Can be repeated, the initrds are concatenated in order | No |
| `linux`  | `cmdline` | The kernel command line, as is                         | No       |
| `efi`    | `path`    | A file reference to the EFI application to chainload   | Yes      |
| `folder` | `default` | Id of the entry selected when the folder is opened     | No       |

The top level `default` can also name an entry inside a folder.

## File references

Files are referenced by their path on the ESP, using backslashes, for example
`\EFI\bread\kernels\bzImage`.

A path can be pinned to a BLAKE3 hash by appending `#blake3:` and the 64 digit
hex digest. Bread will then refuse to use the file if its contents don't match:

```
kernel \EFI\bread\kernels\bzImage#blake3:d7864a…eb93
```

You can get the digest with `b3sum`.

## Example

```
timeout 5
default nixos

folder nixos
  name NixOS
  default nixos-gen-3

  linux nixos-gen-3
    name Generation 3
    kernel \EFI\bread\kernels\bzImage
    initrd \EFI\bread\kernels\initrd
    initrd \EFI\bread\kernels\secrets.initrd
    cmdline init=/nix/store/...-nixos-system/init loglevel=4
  end
end

efi windows
  name Windows
  path \EFI\Microsoft\Boot\bootmgfw.efi
end
```

## Errors

Unknown keys are skipped with a warning, so an older Bread can still read a
config written for a newer one. If a key appears twice, the last one wins.

Anything else that's wrong, like an entry missing a required key, a missing
`end` or a bad hash, stops the boot. Bread shows the error together with the
file and line it comes from, saves its log to `\EFI\bread\bread.log` and reboots
after a key press.

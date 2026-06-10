#!/usr/bin/env -S deno run --allow-read --allow-write --allow-run --allow-env

/**
 * bread-install.ts — NixOS bootloader installer for the bread UEFI bootloader.
 *
 * Usage: bread-install <config-json-path> <current-system-closure>
 *
 * <config-json-path>  Path to a JSON file written by bread.nix containing all
 *                     install-time parameters (see InstallConfig below).
 * <current-system-closure>  The path to the current NixOS system closure
 *                     (passed by NixOS's installBootLoader mechanism; used
 *                     only to confirm we have a valid invocation context).
 *
 * All configuration is passed via the JSON file rather than baked into the
 * script with string substitution, which makes this script a plain file that
 * can be read, tested, and linted without Nix preprocessing.
 */

import { basename, dirname, join } from "jsr:@std/path@^1";
import { join as winJoin } from "jsr:@std/path@^1/windows";
import { ensureDir, exists, move, walk } from "jsr:@std/fs@^1";
import { encodeHex } from "jsr:@std/encoding@^1/hex";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** Shape of the JSON config file written by bread.nix at eval time. */
interface InstallConfig {
  /** Absolute path to the ESP mount point, e.g. "/boot". */
  efiSysMountPoint: string;
  /** Absolute path to the bread.efi binary in the Nix store. */
  efiBinary: string;
  /** Removable-media EFI filename, e.g. "BOOTX64.EFI". */
  efiFile: string;
  /** Seconds before auto-boot; null means wait forever. */
  timeout: number | null;
  /** Whether to remember and restore the last chosen entry. */
  rememberLast: boolean;
  /** Maximum number of generations to expose; null = unlimited. */
  maxGenerations: number | null;
  /** Absolute path to a PSF2 font file, or null to use the built-in font. */
  font: string | null;
  /** Whether efibootmgr may write EFI variables. */
  canTouchEfiVariables: boolean;
  /** Absolute path to the efibootmgr binary (full Nix store path, not PATH-relative). */
  efibootmgrBin: string;
  /** Install to the removable path instead of registering an EFI entry. */
  removable: boolean;
  /** Extra boot entries appended after the NixOS folder. */
  extraEntries: unknown[];
}

/** org.nixos.bootspec.v1 payload. */
interface BootspecV1 {
  system: string;
  init: string;
  initrd?: string;
  initrdSecrets?: string;
  kernel: string;
  kernelParams: string[];
  label: string;
  toplevel: string;
}

/** Parsed, self-contained bootspec for one generation (or specialisation). */
interface BootSpec {
  init: string;
  kernel: string;
  kernelParams: string[];
  label: string;
  toplevel: string;
  initrd?: string;
  initrdSecrets?: string;
  specialisations: Record<string, BootSpec>;
}

const log = (msg: string) => console.log(msg);
const warn = (msg: string) => console.error(`warning: ${msg}`);

const efiPath = (...parts: string[]) => "\\" + winJoin(...parts);
/** Shorthand for paths inside \EFI\nixos\. */
const nixosEfiPath = (filename: string) => efiPath("EFI", "nixos", filename);

const BOOTSPEC_KEY = "org.nixos.bootspec.v1";
const SPEC_KEY = "org.nixos.specialisation.v1";

function parseBootspec(doc: Record<string, unknown>): BootSpec {
  const v1 = doc[BOOTSPEC_KEY] as BootspecV1 | undefined;

  if (!v1) {
    throw new Error(`boot.json is missing "${BOOTSPEC_KEY}"`);
  }

  const rawSpecs = (doc[SPEC_KEY] ?? {}) as Record<
    string,
    Record<string, unknown>
  >;
  const specialisations: Record<string, BootSpec> = {};
  for (const [name, specDoc] of Object.entries(rawSpecs)) {
    specialisations[name] = parseBootspec(specDoc);
  }

  return {
    init: v1.init,
    kernel: v1.kernel,
    kernelParams: v1.kernelParams,
    label: v1.label,
    toplevel: v1.toplevel,
    initrd: v1.initrd,
    initrdSecrets: v1.initrdSecrets,
    specialisations,
  };
}

async function readBootspec(generationDir: string): Promise<BootSpec | null> {
  const bootJsonPath = join(generationDir, "boot.json");
  let raw: string;
  try {
    raw = await Deno.readTextFile(bootJsonPath);
  } catch {
    warn(`boot.json not found at ${bootJsonPath}, skipping generation`);
    return null;
  }

  let doc: Record<string, unknown>;
  try {
    doc = JSON.parse(raw);
  } catch (e) {
    warn(`malformed boot.json at ${bootJsonPath}: ${e}`);
    return null;
  }

  try {
    return parseBootspec(doc);
  } catch (e) {
    warn(`invalid bootspec in ${bootJsonPath}: ${e}`);
    return null;
  }
}

// ---------------------------------------------------------------------------
// Generation discovery
// ---------------------------------------------------------------------------

interface Generation {
  number: number;
  dir: string;
}

async function findGenerations(
  maxGenerations: number | null,
): Promise<Generation[]> {
  const profilesDir = "/nix/var/nix/profiles";
  const results: Generation[] = [];

  for await (const entry of Deno.readDir(profilesDir)) {
    const m = entry.name.match(/^system-(\d+)-link$/);
    if (m) {
      results.push({
        number: parseInt(m[1], 10),
        dir: join(profilesDir, entry.name),
      });
    }
  }

  // Newest generation first.
  results.sort((a, b) => b.number - a.number);

  return maxGenerations !== null && maxGenerations > 0
    ? results.slice(0, maxGenerations)
    : results;
}

// ---------------------------------------------------------------------------
// Atomic file helpers
// ---------------------------------------------------------------------------

/** Write dst atomically via a sibling temp file: writeFn → fsync → rename. */
async function atomicWrite(
  dst: string,
  writeFn: (tmp: string) => Promise<void>,
): Promise<void> {
  await ensureDir(dirname(dst));
  const tmpPath = await Deno.makeTempFile({ dir: dirname(dst) });
  try {
    await writeFn(tmpPath);
    // fsync before rename so data is durable on the FAT32 ESP.
    await using f = await Deno.open(tmpPath, { write: true });
    await f.syncData();
    await move(tmpPath, dst, { overwrite: true });
  } catch (e) {
    try {
      await Deno.remove(tmpPath);
    } catch { /* ignore */ }
    throw e;
  }
}

const atomicWriteText = (dst: string, content: string) =>
  atomicWrite(dst, (tmp) => Deno.writeTextFile(tmp, content));

const atomicCopy = (src: string, dst: string) =>
  atomicWrite(dst, (tmp) => Deno.copyFile(src, tmp));

/**
 * Atomic copy, but no-op if the destination already exists (content-addressed
 * files: same destination name means identical content).
 *
 * Two concurrent installs may both observe !exists and proceed; the last
 * rename wins, which is safe because both would produce identical content.
 */
async function atomicCopyIfAbsent(src: string, dst: string): Promise<void> {
  if (await exists(dst)) return;
  await atomicCopy(src, dst);
}

// ---------------------------------------------------------------------------
// Content-addressed file copy
// ---------------------------------------------------------------------------

/**
 * Copy a kernel/initrd file to nixosDir using a hash of its store path as the
 * name.  Returns the destination path and records it in `referenced`.
 *
 * We hash the resolved store path string rather than the file content because
 * Nix's content-addressing guarantees that the store path is a reliable proxy
 * for file identity: the same path always contains the same bytes.  Hashing
 * the path string is orders of magnitude faster than hashing a 10 MB kernel.
 */
async function copyKernelFile(
  srcPath: string,
  nixosDir: string,
  suffix: string,
  referenced: Set<string>,
): Promise<string> {
  const resolved = await Deno.realPath(srcPath);
  const hashBuf = await crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode(resolved),
  );
  const h = encodeHex(hashBuf).slice(0, 16);
  const dst = join(nixosDir, `${h}${suffix}`);
  await atomicCopyIfAbsent(resolved, dst);
  referenced.add(dst);
  return dst;
}

// ---------------------------------------------------------------------------
// Boot entry building
// ---------------------------------------------------------------------------

interface LinuxEntry {
  type: "linux";
  id: string;
  name: string;
  kernel: string;
  initrd: string[];
  cmdline?: string;
}

interface FolderEntry {
  type: "folder";
  id: string;
  name: string;
  default: string;
  entries: (LinuxEntry | FolderEntry)[];
}

type BootEntry = LinuxEntry | FolderEntry;

async function buildLinuxEntry(
  id: string,
  name: string,
  spec: BootSpec,
  nixosDir: string,
  referenced: Set<string>,
): Promise<LinuxEntry> {
  const kernelDst = await copyKernelFile(
    spec.kernel,
    nixosDir,
    ".efi",
    referenced,
  );
  const initrdPaths: string[] = [];

  if (spec.initrd) {
    const dst = await copyKernelFile(
      spec.initrd,
      nixosDir,
      ".initrd",
      referenced,
    );
    initrdPaths.push(nixosEfiPath(basename(dst)));
  }

  if (spec.initrdSecrets) {
    // initrdSecrets is an executable that writes secrets to stdout (or a path).
    // We need to run it to produce the actual secrets initrd, then copy that.
    const secretsDst = join(
      nixosDir,
      `${basename(spec.toplevel)}-secrets.initrd`,
    );
    const ok = await runInitrdSecrets(spec.initrdSecrets, secretsDst);
    if (ok) {
      referenced.add(secretsDst);
      initrdPaths.push(nixosEfiPath(basename(secretsDst)));
    }
  }

  const cmdlineParts = spec.init ? [`init=${spec.init}`] : [];
  cmdlineParts.push(...spec.kernelParams);

  const entry: LinuxEntry = {
    type: "linux",
    id,
    name,
    kernel: nixosEfiPath(basename(kernelDst)),
    initrd: initrdPaths,
  };
  if (cmdlineParts.length > 0) {
    entry.cmdline = cmdlineParts.join(" ");
  }
  return entry;
}

/**
 * Run the initrdSecrets script for a generation, writing output to dstPath.
 * Returns true on success, false on failure (with a warning printed).
 */
async function runInitrdSecrets(
  secretsScript: string,
  dstPath: string,
): Promise<boolean> {
  if (!await exists(secretsScript)) {
    return false; // script absent — older generation without secrets, skip silently
  }

  const tmpPath = await Deno.makeTempFile({ dir: dirname(dstPath) });
  try {
    const cmd = new Deno.Command(secretsScript, {
      args: [tmpPath],
      stdout: "null",
      stderr: "inherit",
    });
    const { code } = await cmd.output();
    if (code !== 0) {
      warn(
        `initrdSecrets script exited with code ${code}, skipping secrets initrd`,
      );
      try {
        await Deno.remove(tmpPath);
      } catch { /* ignore */ }
      return false;
    }

    if (!await exists(tmpPath, { isFile: true })) {
      // Some scripts output to stdout; that usage isn't supported here.
      warn(`initrdSecrets script did not produce a file at ${tmpPath}`);
      try {
        await Deno.remove(tmpPath);
      } catch { /* ignore */ }
      return false;
    }

    // fsync before rename for FAT32 durability, consistent with atomicCopy.
    await using f = await Deno.open(tmpPath, { write: true });
    await f.syncData();
    await move(tmpPath, dstPath, { overwrite: true });
    return true;
  } catch (e) {
    warn(`failed to run initrdSecrets script: ${e}`);
    try {
      await Deno.remove(tmpPath);
    } catch { /* ignore */ }
    return false;
  }
}

async function buildGenerationEntries(
  generations: Generation[],
  nixosDir: string,
  referenced: Set<string>,
): Promise<{ entries: BootEntry[]; defaultId: string }> {
  // Build all generations concurrently; Promise.all preserves sorted order.
  const built = await Promise.all(
    generations.map(async (gen) => {
      const spec = await readBootspec(gen.dir);
      if (!spec) return null;

      const genId = `nixos-gen-${gen.number}`;
      const mainEntry = await buildLinuxEntry(
        genId,
        `Generation ${gen.number}`,
        spec,
        nixosDir,
        referenced,
      );

      const specEntries: LinuxEntry[] = [];
      for (
        const [specName, specData] of Object.entries(spec.specialisations)
          .sort()
      ) {
        specEntries.push(
          await buildLinuxEntry(
            `nixos-gen-${gen.number}-spec-${specName}`,
            specName,
            specData,
            nixosDir,
            referenced,
          ),
        );
      }

      return { gen, genId, mainEntry, specEntries };
    }),
  );

  const entries: BootEntry[] = [];
  let defaultId = "";

  for (const result of built) {
    if (!result) continue;
    const { gen, genId, mainEntry, specEntries } = result;
    const isFirst = defaultId === "";

    if (isFirst) mainEntry.name += " (current)";

    if (specEntries.length > 0) {
      const folderId = `nixos-gen-${gen.number}-folder`;
      if (isFirst) defaultId = folderId;
      entries.push({
        type: "folder",
        id: folderId,
        name: `Generation ${gen.number}`,
        default: genId,
        entries: [mainEntry, ...specEntries],
      });
    } else {
      if (isFirst) defaultId = genId;
      entries.push(mainEntry);
    }
  }

  return { entries, defaultId };
}

// ---------------------------------------------------------------------------
// Garbage-collect stale ESP files
// ---------------------------------------------------------------------------

async function gcNixosDir(
  nixosDir: string,
  referenced: Set<string>,
): Promise<void> {
  if (!await exists(nixosDir, { isDirectory: true })) return;

  for await (
    const entry of walk(nixosDir, {
      maxDepth: 1,
      includeFiles: true,
      includeDirs: false,
    })
  ) {
    if (!referenced.has(entry.path)) {
      log(`removing stale ESP file: ${entry.path}`);
      await Deno.remove(entry.path);
    }
  }
}

// ---------------------------------------------------------------------------
// EFI variable registration (efibootmgr)
// ---------------------------------------------------------------------------

async function registerEfiEntry(esp: string, efibootmgr: string): Promise<void> {
  let partDev: string;
  try {
    partDev = await runOutput("findmnt", [
      "-n",
      "-o",
      "SOURCE",
      "--target",
      esp,
    ]);
  } catch {
    warn(
      "could not determine ESP block device; skipping EFI entry registration",
    );
    return;
  }
  if (!partDev) {
    warn(
      "could not determine ESP block device; skipping EFI entry registration",
    );
    return;
  }

  let pkname: string;
  let partNum: string;
  try {
    const out = await runOutput("lsblk", ["-no", "PKNAME,PARTN", "--raw", partDev]);
    [pkname, partNum] = out.split(/\s+/);
  } catch {
    warn(
      "could not determine ESP disk/partition; skipping EFI entry registration",
    );
    return;
  }
  if (!pkname || !partNum) {
    warn(
      "could not determine ESP disk/partition; skipping EFI entry registration",
    );
    return;
  }
  const diskDev = `/dev/${pkname}`;

  let efiOut: string;
  try {
    efiOut = await runOutput(efibootmgr, []);
  } catch {
    warn("efibootmgr not found or failed; skipping EFI entry registration");
    return;
  }

  const existing = [...efiOut.matchAll(/Boot([0-9a-fA-F]{4})\*? bread\b/g)]
    .map((m) => m[1]);

  try {
    // Create the new entry first so the system always has a working EFI entry.
    await run(efibootmgr, [
      "--create",
      "--disk",
      diskDev,
      "--part",
      partNum,
      "--loader",
      "\\EFI\\bread\\bread.efi",
      "--label",
      "bread",
    ]);
    // Delete stale entries only after the new one is live.
    for (const bootNum of existing) {
      await run(efibootmgr, ["-b", bootNum, "-B"]);
    }
    log(
      existing.length > 0
        ? "updated EFI boot entry for bread"
        : "created EFI boot entry for bread",
    );
  } catch (e) {
    warn(`efibootmgr failed: ${e}`);
  }
}

// ---------------------------------------------------------------------------
// Main install logic
// ---------------------------------------------------------------------------

async function install(cfg: InstallConfig): Promise<void> {
  const esp = cfg.efiSysMountPoint;
  const nixosDir = join(esp, "EFI", "nixos");
  const breadDir = join(esp, "EFI", "bread");

  await ensureDir(nixosDir);
  await ensureDir(breadDir);

  // --- Discover generations ---
  const generations = await findGenerations(cfg.maxGenerations);
  if (generations.length === 0) {
    warn("no NixOS generations found");
  }

  // --- Build boot entries and copy kernel/initrd files ---
  const referenced = new Set<string>();
  const { entries: genEntries, defaultId } = await buildGenerationEntries(
    generations,
    nixosDir,
    referenced,
  );

  // --- GC stale files from EFI/nixos/ ---
  await gcNixosDir(nixosDir, referenced);

  // --- Wrap all generation entries in a NixOS folder ---
  const nixosFolder: FolderEntry = {
    type: "folder",
    id: "nixos",
    name: "NixOS",
    default: defaultId,
    entries: genEntries,
  };

  // --- Build bread.json config ---
  const config: Record<string, unknown> = {
    timeout: cfg.timeout,
    remember_last: cfg.rememberLast,
    default: "nixos",
    entries: [nixosFolder, ...cfg.extraEntries],
  };

  // --- Handle font ---
  const fontDst = join(breadDir, "bread.psf");
  if (cfg.font) {
    // Copy atomically so a power failure mid-copy cannot leave a corrupt font.
    await atomicCopy(cfg.font, fontDst);
    config["font"] = efiPath("EFI", "bread", "bread.psf");
  } else {
    // Remove stale font if a previous install had one.
    try {
      await Deno.remove(fontDst);
    } catch { /* not present, that's fine */ }
  }

  // --- Write bread.json atomically ---
  const configPath = join(breadDir, "bread.json");
  await atomicWriteText(configPath, JSON.stringify(config, null, 2) + "\n");
  log(`wrote ${configPath}`);

  // --- Install bread.efi atomically ---
  // Always overwrite: the binary changes with every bread package update.
  if (cfg.removable) {
    const bootDir = join(esp, "EFI", "BOOT");
    await ensureDir(bootDir);
    const dst = join(bootDir, cfg.efiFile);
    await atomicCopy(cfg.efiBinary, dst);
    log(`installed ${dst}`);
  } else {
    const dst = join(breadDir, "bread.efi");
    await atomicCopy(cfg.efiBinary, dst);
    log(`installed ${dst}`);
    if (cfg.canTouchEfiVariables) {
      await registerEfiEntry(esp, cfg.efibootmgrBin);
    }
  }
}

async function run(cmd: string, args: string[]): Promise<void> {
  const { code } = await new Deno.Command(cmd, {
    args,
    stdout: "null",
    stderr: "inherit",
  }).output();
  if (code !== 0) throw new Error(`${cmd} exited with code ${code}`);
}

async function runOutput(cmd: string, args: string[]): Promise<string> {
  const { code, stdout, stderr } = await new Deno.Command(cmd, {
    args,
    stdout: "piped",
    stderr: "piped",
  }).output();
  if (code !== 0) {
    const errText = new TextDecoder().decode(stderr).trim();
    throw new Error(
      `${cmd} exited with code ${code}${errText ? `: ${errText}` : ""}`,
    );
  }
  return new TextDecoder().decode(stdout).trim();
}

const args = Deno.args;

if (args.length < 2) {
  console.error(
    "usage: bread-install <config-json-path> <current-system-closure>",
  );
  Deno.exit(1);
}

const configPath = args[0];
// args[1] is the current-system closure path (required by NixOS convention,
// but we derive everything from /nix/var/nix/profiles ourselves).

let cfg: InstallConfig;
try {
  const raw = await Deno.readTextFile(configPath);
  cfg = JSON.parse(raw) as InstallConfig;
} catch (e) {
  console.error(
    `error: could not read install config at ${configPath}: ${
      e instanceof Error ? e.message : e
    }`,
  );
  Deno.exit(1);
}

try {
  await install(cfg);
} catch (e) {
  console.error(`error: ${e instanceof Error ? e.message : e}`);
  Deno.exit(1);
}

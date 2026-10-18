# Linux AppImage license data

The Linux AppImage brings system libraries from Ubuntu 22.04 with it (GTK, WebKitGTK, GLib and
everything linuxdeploy copies into the AppDir) and the tools that make up the AppImage. Their
licenses are part of `THIRD_PARTY_LICENSES` (T3.23). The macOS and Windows builds and the `.deb`
package do not contain these libraries.

| File | Content | Kept by |
|---|---|---|
| `libs.txt` | every file of the AppImage that comes from an Ubuntu package, with package, version, source package and source version | `scripts/appimage-libs.sh` |
| `copyright/<source>` | the Debian copyright file of each source package, byte for byte | `scripts/appimage-libs.sh` |
| `sources.tsv` | license summary of the source packages whose copyright file is not in the machine readable Debian format | hand |
| `other.tsv`, `other/` | the AppImage runtime, AppRun and the GTK hook: license, copyright, source, license text | hand |

`node scripts/third-party-licenses.mjs` reads these files on every platform; it needs no Linux
machine. The texts the copyright files refer to in `/usr/share/common-licenses/` come from
`scripts/license-texts/` (SPDX License List, see the README there).

## How the data stays current

Only the Ubuntu machine that builds the AppImage knows which files it bundles. The linux job of
`.github/workflows/release.yml` runs `./scripts/appimage-libs.sh --check` on the built AppImage
after the build. It fails when a file, package, version or copyright file differs, prints the
difference, and uploads the updated data as the artifact `appimage-license-data`. The script
leaves out only the files it names in `not_from_a_package` (DimoMark's own files, the AppImage
tools of `other.tsv`, the copyright files and generated caches); any other file without a
package fails the check. GitHub refreshes its Ubuntu runner image about every week, which
changes package versions and sometimes the set of installed packages, so expect this check to
ask for an update now and then.

To update:

1. Download the artifact `appimage-license-data` of the failed run. Replace `libs.txt` and the
   whole `copyright/` folder with its content (delete the old folder first, so copyright files
   of packages that are gone disappear), or run `./scripts/appimage-libs.sh --update <AppImage>`
   on Ubuntu 22.04. Files from the artifact come back executable: run
   `chmod 644 scripts/appimage/copyright/*` so git sees only real changes.
2. Run `node scripts/third-party-licenses.mjs`. It stops when `sources.tsv` needs a change: a
   new source package whose copyright file is not in the machine readable Debian format needs a
   line, a line of a package that is gone or whose file is machine readable must go.
3. Commit.

`other.tsv` describes tools that tauri-cli downloads while it builds the AppImage. Check it when
tauri-cli is updated.

## Sources of the first version

The first version (2026-10-17) was not made by the script, because no Ubuntu machine was at
hand: the file list was read from the AppImage of the release candidate `v0.1.0-rc.2`, packages
and versions from the Ubuntu 22.04 package indexes (`jammy`, `jammy-updates`,
`jammy-security`, newest version), copyright files from `changelogs.ubuntu.com` (the 17
copyright files inside the AppImage are byte for byte the same), the files in `other/` from the
projects' repositories. The first run of the release job checks it against the runner; the
runner may have older preinstalled versions, then update as above.

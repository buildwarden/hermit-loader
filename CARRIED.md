# Carried by BuildWarden

This branch, `bw/v0.5.7`, is Hermit's loader at tag `v0.5.7` plus the
commits BuildWarden carries for its relay
([buildwarden-relay](https://github.com/buildwarden/buildwarden-relay)).
`main` mirrors upstream and carries nothing.

Each carried commit is one of two classes:

- **Upstream-bound:** meant for `hermit-os/loader`, and dropped once it
  lands there.
- **Deviation:** carried by design, whether or not upstream takes it.

On each upstream release, a new `bw/<tag>` is cut from the release tag and
these commits are replayed. Commits that landed upstream are dropped, and
any that conflicted are re-reviewed. Nothing is submitted upstream without
Jeff's approval of that submission, and AI assistance is disclosed when it
is. A rejected or stalled PR turns its commit into a deviation; it never
blocks the relay.

| Commit | Class | Upstream status | What |
|---|---|---|---|
| `bd578a1` aarch64: an aarch64-vz target for Apple's Virtualization framework | Deviation, until split | Not submitted | The arm64 Image header and the fixed link address at VZ's RAM base are VZ-only. Keeping `x0`, copying the device tree below the kernel, and the null console are generic, and are to be split into their own commit and offered upstream. A position-independent loader would replace the fixed link address |

Not carried: the boot checkpoints used to trace VZ boots
(`spike/patches/loader-vz-checkpoints.patch` in buildwarden-relay). They are
a debugging tool, not a change to ship.

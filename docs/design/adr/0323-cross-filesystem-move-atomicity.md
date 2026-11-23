# 0323 — Cross-filesystem moves copy to a sibling, rename, then remove the source

## Status

Accepted (2026-11-22). Implements [design 27](../27-stability-and-daily-driver.md)
§3.5 and closes S-12.

## Context

`moveFilesIntoFolder` tried `rename(2)` first, but its fallback for a
directory created the destination folder and moved children one at a time,
then removed the source tree. A failure partway through left some children
at the destination, some at the source, and returned `false`. The Dock drop
path queues this same core, so a cross-filesystem drop could visibly split a
folder. A directory rename cannot be atomic across mount points.

## Decision

1. **Same filesystem: one `rename(2)`.** Only `EXDEV` selects the fallback;
   any other rename error is reported with the source untouched.
2. **Cross filesystem: hidden temp sibling, copy, rename, remove source.**
   The temp is created in the destination folder, so the final rename is on
   one filesystem and atomic. Files, directories, and symlinks are copied;
   symlinks stay links.
3. **Metadata is best-effort.** Permissions and modification times are copied
   where the destination filesystem allows. A metadata failure does not turn
   a complete copy into a failed move.
4. **Failure removes the temp and leaves the source complete.** The temp is
   made writable first so a read-only directory copied from the source can
   still be deleted. The destination name is reserved by the existing
   generated-name helper before the copy starts; a broken symlink counts as
   occupied. A retry therefore reuses the original name instead of creating
   `name.1` beside a half-moved `name`.
5. **Source removal is cleanup after the destination is complete.** If it
   fails, the move stays successful and the leftover is logged. Deleting a
   good destination to restore a partially removed source would lose data.
6. **A directory cannot move into its own subtree.** The copy would otherwise
   create the temp inside the source and recurse forever.
7. **Deterministic test seams.** `FileMoveOptions::forceCrossFilesystem`
   selects the fallback without a second mount, and `failAfterEntries` stops
   the copy mid-tree so tests can prove rollback.

Rejected alternatives:

- **Keep moving children one at a time.** It cannot be all-or-nothing.
- **Copy directly to the final name.** A failure leaves a visible partial
  destination and a retry sees a collision.
- **Put the temp on the source filesystem.** The final rename would still
  cross the mount.
- **Follow symlinks while copying.** A move would change a link into its
  target.
- **Roll back the destination when source removal fails.** The source may
  already be partly removed; the destination is the only complete copy.

## Consequences

- A cross-filesystem move needs free space for a second copy and is not
  interruptible mid-copy; the mutation worker stays serialized.
- Hard links, ACLs, extended attributes, and ownership are not preserved.
  Permissions and mtime are the contract for this gate.
- A source-removal failure leaves a duplicate rather than failing the move;
  the warning is the only user-invisible trace.
- Files copy/move/paste (T330) and undo (T331) must reuse this core instead
  of adding another per-child loop.

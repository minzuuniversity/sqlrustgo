#!/usr/bin/env python3
"""#4956 A/B: toggle the commit-path flush in WalStorage::commit_transaction.

  apply  — remove the flush (pre-#4956 behaviour: flush deferred to the
           caller's flush() / sweeper)
  revert — put it back exactly as committed

Used to build the `before` binary. The only intended difference between
the two server binaries is this 3-line block.
"""
import sys

PATH = "crates/storage/src/wal_storage.rs"
MARKER = "        // [PERF-4956 A/B] flush removed from the commit path\n"

BLOCK = """        // #4946: the deferred-flush design assumed "WAL is the source of
        // truth, so replay restores the data without a snapshot". That
        // only holds if the WAL entries survive — but the truncation
        // below deletes every entry below the checkpoint, and the
        // snapshot they are supposed to have been folded into is written
        // *later* (by the caller's `flush()`). In that window a crash
        // loses the data on both paths: no snapshot, no WAL entry.
        //
        // Order is therefore load-bearing: flush first, truncate second.
        // `FileStorage::flush` is incremental (`save_table_window` only
        // rewrites rows appended since `last_saved`), so the extra cost
        // on the commit path is proportional to what this transaction
        // actually wrote, not to the table size.
        //
        // Errors are propagated rather than swallowed: acknowledging a
        // commit whose snapshot failed to write is the exact failure
        // mode this issue reports.
        if commit_lsn > 0 {
            self.inner_mut().flush()?;
        }

"""


def main() -> int:
    mode = sys.argv[1]
    src = open(PATH).read()

    if mode == "apply":
        if BLOCK not in src:
            print("FAIL: block not found verbatim — refusing to guess")
            return 1
        if src.count(BLOCK) != 1:
            print("FAIL: block is not unique (%d matches)" % src.count(BLOCK))
            return 1
        open(PATH, "w").write(src.replace(BLOCK, MARKER))
        print("APPLIED: commit-path flush removed")
        return 0

    if mode == "revert":
        if src.count(MARKER) != 1:
            print("FAIL: marker not found exactly once")
            return 1
        open(PATH, "w").write(src.replace(MARKER, BLOCK))
        print("REVERTED: commit-path flush restored")
        return 0

    print("usage: ab_patch.py apply|revert")
    return 2


if __name__ == "__main__":
    sys.exit(main())

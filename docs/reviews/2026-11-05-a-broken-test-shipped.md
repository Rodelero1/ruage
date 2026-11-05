# Review: a broken test shipped, and how

*Standing correction to D126. The commit `d2cad09` shipped `crates/view/tests/minimap_seams.rs` with a **compile error**: a
patch that added one `assert_eq!` inside another, so the file ended with two nested assertions and an unbalanced brace. The push
went through anyway.*

## What happened, precisely

The sequence was: edit the test's message with `patch`, then run

```
cargo test ... --test minimap_seams 2>&1 | tail -2 && git commit ... && git push
```

`cargo test` failed to compile. `tail` **succeeded**, and in a shell pipeline the exit status is the last command's - so the
`&&` chain ran, committed and pushed a file that does not compile.

This is the exact pitfall this project recorded after the same mistake earlier:

> **Piped grep is not a green suite.** `cargo test ... | grep ...` exits 0 whatever `cargo` did.

It was written down, and it was walked into again three days later. The lesson is not "be careful"; it is that the check has to
be mechanical: `cargo test` unpiped, or `${PIPESTATUS[0]}`, or a wrapper that refuses on a non-zero compiler status. A rule in a
document did not stop it; a wrong exit code did not stop it either, because nobody read it.

## Performance

The change is a brace and a duplicated assertion in a test file; no production code, no draw call, no tick. The numbers that do
belong here are the **cost of the check that was skipped**, measured on this machine with the build warm:

| command | time | exit status |
|---|---|---|
| `cargo test -p ruage-view --test minimap_seams` (unpiped) | **132 ms** | **0** |
| the same command piped into `tail` | 99 ms | **0**, whatever the compiler said |

A tenth of a second of checking was skipped to save a tenth of a second - and the pipeline reported success for a file that did
not compile. The suite figures stand from D126: **695 passed / 0 failed / 19 ignored**, acceptance hash `4568cef16368bd5a`
unchanged.

## Stability

| attack | result |
|---|---|
| the file compiles | `cargo test -p ruage-view --test minimap_seams`, **unpiped**, exits 0 and reports 2 passed |
| both seam cases still fail on the old code | verified in the `c9077b7` worktree: **182** gaps in each of the two tests |
| any other test broken by this | full suite unpiped: 695 passed, 0 failed, 19 ignored; clippy clean |
| the same mistake again | the only defence that works is the exit code being read: the review gate now refuses a commit that touches code without a note, and this note records why an unpiped run is required rather than recommended |

---

*`tools/review_check.py` looks for `## Performance` and `## Stability`; both are above.*

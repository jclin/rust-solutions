# rust-solutions

My solutions to the challenge exercises in [*Command-Line Rust*](https://www.oreilly.com/library/view/command-line-rust/9781098109424/) by Ken Youens-Clark. Each chapter asks you to write a Rust version of a common Unix tool, and this repo holds my implementation of each one.

The author's own solutions are at [kyclark/command-line-rust](https://github.com/kyclark/command-line-rust).

## Programs

| Chapter | Program | Based on |
|---------|---------|----------|
| 1 | `hello` | hello world |
| 2 | `echor` | `echo` |
| 3 | `catr` | `cat` |
| 4 | `headr` | `head` |
| 5 | `wcr` | `wc` |
| 6 | `uniqr` | `uniq` |
| 7 | `findr` | `find` |
| 8 | `cutr` | `cut` |
| 9 | `grepr` | `grep` |
| 10 | `commr` | `comm` |
| 11 | `tailr` | `tail` |
| 12 | `fortuner` | `fortune` |
| 13 | `calr` | `cal` |
| 14 | `lsr` | `ls` |

## Running

Each program is its own Cargo project. To build and test one:

```sh
cd catr
cargo test
cargo run -- --help
```

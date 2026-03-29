# collision probability

cli tool that answers: given my id format, how likely are collisions to `n`? (collision probability)

uses the [Birthday problem](https://en.wikipedia.org/wiki/Birthday_problem) with math to compute exact collision probabilities for any character-based id scheme (mostly)

> assumes uniform random generation

## build

```
cargo build --release
```

## usage

```
cargo run --release -- '<spec>' <command> [args]
```

### commands

| command              | description                                      |
| -------------------- | ------------------------------------------------ |
| `space`              | print the total id space size and spec breakdown |
| `sets <count>`       | generate up to 10 example ids                    |
| `collision <n1> ...` | compute collision probabilities for given sizes  |

`--json` is available as a global flag on all commands.

### space spec format

groups are separated by `;` and each group is `<chars>|<positions>` (positions defaults to 1 if omitted).

ranges are supported: `a-z`, `A-Z`, `0-9`, etc.

| spec                   | meaning                         | space size    |
| ---------------------- | ------------------------------- | ------------- |
| `a-z\|4`               | 4 lowercase letters             | 26^4          |
| `a-zA-Z\|4;0-9\|4`     | 4 letters + 4 digits            | 52^4\*10^4    |
| `a-zA-Z0-9\|20`        | 20 alphanumeric chars           | 62^20         |
| `!@#`                  | 1 symbol from `!@#`             | 3             |
| `a-z\|2;!@#\|1;0-9\|3` | 2 letters + 1 symbol + 3 digits | 26^2\*3\*10^3 |

if your spec requires `|` as a character, you have to add the `<positions>`. example:
instead of:

```
cargo run --release -- '#|' collision 1
```

you run:

```
cargo run --release -- '#||1' collision 1
```

### output columns

| column                | definition                                                        |
| --------------------- | ----------------------------------------------------------------- |
| **P(collision)**      | Birthday problem probability that at least two of `n` ids collide |
| **Odds**              | Same probability as "1 in X"                                      |
| **Unique on 1st try** | chance a new random one is unique in `n` ids                      |
| **Avg retries**       | Expected random attempts to generate one more unique id           |

## examples

```
$ cargo run --release -- 'a-zA-Z|4;0-9|4' space

Space: 52^4 * 10^4 = 73,116,160,000 possible IDs
Spec:
    - a-zA-Z|4 = 4 characters in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
    - 0-9|4 = 4 characters in "0123456789"
```

```
$ cargo run --release -- 'a-zA-Z|4;0-9|4' sets 3

Example IDs:
  rKwB4829
  LpAn0173
  YdQx8461
```

```
$ cargo run --release -- 'a-zA-Z|4;0-9|4' collision 1000 10000 100000 1000000

Space: 52^4 * 10^4 = 73,116,160,000 possible IDs

╭───────────┬────────────────────┬──────────────┬───────────────────┬─────────────╮
│ n         │ P(collision)       │ Odds         │ Unique on 1st try │ Avg retries │
├───────────┼────────────────────┼──────────────┼───────────────────┼─────────────┤
│ 1,000     │ 6.83e-6 (0.0007%)  │ 1 in 146,379 │ 100.0000%         │ 1.0000      │
│ 10,000    │ 6.84e-4 (0.0684%)  │ 1 in 1,462   │ 100.0000%         │ 1.0000      │
│ 100,000   │ 6.61e-2 (6.6098%)  │ 1 in 15      │ 99.9999%          │ 1.0000      │
│ 1,000,000 │ 9.99e-1 (99.8928%) │ 1 in 1.0     │ 99.9986%          │ 1.0000      │
╰───────────┴────────────────────┴──────────────┴───────────────────┴─────────────╯
```

## how it works

instead of looping `n` times, the probability is computed in O(1) using the log-gamma identity:

```
ln(P(no collision)) = ln_gamma(S+1) - ln_gamma(S-n+1) - n * ln(S)
```

> this means computing probabilities for n=10,000,000 or n=1,000,000,000 is instant.

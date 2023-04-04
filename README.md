<div align="center">
  <img height="250" width="250" src="https://github.com/Ackee-Blockchain/trdelnik/raw/master/assets/Badge_Trdelnik.png" alt="Trdelnik Logo"/>

  # Trdelník

  <a href="https://discord.gg/x7qXXnGCsa">
    <img src="https://discordapp.com/api/guilds/867746290678104064/widget.png?style=banner2" width="250" title="AckeeBlockchain/Trdelnik discord" alt="Ackee Blockchain Discord invitation">
  </a>

  developed by [Ackee Blockchain](https://ackeeblockchain.com)

  [![Crates.io](https://img.shields.io/crates/v/trdelnik-cli?label=CLI)](https://crates.io/crates/trdelnik-cli)
  [![Crates.io](https://img.shields.io/crates/v/trdelnik-test?label=Test)](https://crates.io/crates/trdelnik-test)
  [![Crates.io](https://img.shields.io/crates/v/trdelnik-client?label=Client)](https://crates.io/crates/trdelnik-client)
  [![Crates.io](https://img.shields.io/crates/v/trdelnik-explorer?label=Explorer)](https://crates.io/crates/trdelnik-explorer)
  <br />
  [![lint](https://github.com/Ackee-Blockchain/trdelnik/actions/workflows/lint.yml/badge.svg)](https://github.com/Ackee-Blockchain/trdelnik/actions/workflows/lint.yml)
  [![test-examples-turnstile](https://github.com/Ackee-Blockchain/trdelnik/actions/workflows/test-examples-turnstile.yml/badge.svg)](https://github.com/Ackee-Blockchain/trdelnik/actions/workflows/test-examples-turnstile.yml)

</div>

Trdelník is Rust based testing framework providing several convenient developer tools for testing Solana programs written in [Anchor](https://github.com/project-serum/anchor).

- **Trdelnik client** - build and deploy an Anchor program to a local cluster and run a test suite against it;
- **Trdelnik console** - built-in console to give developers a command prompt for quick program interaction;
- **Trdelnik fuzz** - property-based and stateful testing;
- **Trdelnik explorer** - exploring a ledger changes.

<div align="center">
  <img src="https://github.com/Ackee-Blockchain/trdelnik/raw/master/assets/demo.svg" alt="Trdelnik Demo" />
</div>

## Dependencies

- Install [Rust](https://www.rust-lang.org/tools/install) (`nightly` release)
- Install [Solana tool suite](https://docs.solana.com/cli/install-solana-cli-tools) (`stable` release)
- Install [Anchor](https://book.anchor-lang.com/chapter_2/installation.html)

## Installation

```shell
cargo install trdelnik-cli

# or the specific version

cargo install --version <version> trdelnik-cli
```

### Documentation

[Trdelnik docs](https://reastyn.github.io/trdelnik/book/motivation.html)

### Supported versions

- We support `Anchor` and `Solana` versions specified in the table below.

| Trdelnik CLI |  Anchor   |   Solana |
|--------------|:---------:|---------:|
| `latest`     | `~0.27.*` | `>=1.15` |
| `v0.3.0`     | `~0.25.*` | `>=1.10` |
| `v0.2.0`     | `~0.24.*` |  `>=1.9` |

- _We are exploring a new versions of Anchor, please make sure you only use the supported versions. We are working on it :muscle:_

## Roadmap

- [x] Q1/22 Trdelnik announcement at Solana Hacker House Prague
  - [x] Trdelnik client available for testing
- [x] Q2/22 Trdelnik explorer available
- [x] Q2/22 Trdelnik client and explorer introduced at Solana Hacker House Barcelona
- [ ] Q3/22 Trdelnik console available
- [ ] Q4/22 Trdelnik fuzz available

## Awards

**Marinade Community Prize** - winner of the [Marinade grant](https://solana.blog/riptide-hackathon-winners/) for the 2022 Solana Riptide Hackathon.

## Contribution

Thank you for your interest in contributing to Trdelník! Please see the [CONTRIBUTING.md](./CONTRIBUTING.md) to learn how.

## License

This project is licensed under the [MIT license](https://github.com/Ackee-Blockchain/trdelnik/blob/master/LICENSE).

## University and investment partners

- [Czech technical university in Prague](https://www.cvut.cz/en)
- [Ackee](https://www.ackee.cz/)
- [Rockaway Blockchain Fund](https://rbf.capital/)

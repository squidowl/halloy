# Contributing

Halloy! If you don't know it yet, but "Halløj" is the a danish "Hello" or "Hi". Thank you for visiting our development guide.

You have various options for contributing to Halloy. Before you start working on Halloy, read the topics “[Be nice](#be-nice)”, "[Goals](#goals)" and "[Licensing](#licensing)"

## Be nice

**TBD:** Do we need such a policy? Do we want to adopt the [Rust CoC](https://www.rust-lang.org/policies/code-of-conduct)?

## Goals

- **simple:** The UI and functions should be intuitive and accessible.
- **fast:** *TBD*
- **eternal:** Halloy will be the last actively used software at the end of time.

## Licensing

Halloy is released under the **GNU General Public License v3.0 or later (GPL‑3.0‑or‑later)**, which means you’re free to use, modify, and distribute the code — but any derivative work must also carry the same copyleft terms.

By contributing code, you automatically license your contributions under GPL‑3.0‑or‑later. No Contributor License Agreement (CLA) needed — opening a pull request is enough.

Make sure any third‑party libraries or snippets you add are compatible with GPL‑3.0‑or‑later.

You’ll find the full license text in the [LICENSE](https://github.com/squidowl/halloy/blob/main/LICENSE) file, and it’s also declared in each Cargo.toml (license = "GPL-3.0-or-later").

## Contributing code

### Prerequisites

You need to be familar with

- **Rust:** The programming language in which Halloy is developed. Visit [Learn Rust](https://www.rust-lang.org/learn) to get more information.
- **iced:** GUI library which Halloy use for its UI. Go to the [iced homepage](https://book.iced.rs) to learn more about it.
- **Tokio:** Tokio is an asynchronous runtime for the Rust programming language. Learn more at the [Tokio](https://tokio.rs) project page.
- **Git:** Distributed version control system which helps to collobarate on the Halloy source code.
- **GitHub:** The DevOps platform that we use. You can visit [Halloy](https://github.com/squidowl/halloy) at GitHub to get the latest source code, reporting bugs and contributing to Halloy.
- **IRC:** Halloy has been made for Internet Relay Chat. You can find the latest information about the protocol specification on the [IRCv3 page](https://ircv3.net).

### Architecture

TODO

### Codebase

TODO

### Coding-Standards & Guidelines

To keep things tidy, readable, and high-quality across Halloy — from source code to docs and config files — we follow a few simple rules. Sticking to these makes collaboration smoother, helps avoid unnecessary diffs, and ensures that contributions fit in nicely with the rest of the project.

If you're thinking about opening a PR, take a minute to go through the standards below. It’ll save everyone time — including you.

#### Formatting & Linting

##### Rust

We use [rustfmt](https://github.com/rust-lang/rustfmt) to keep the Rust codebase clean and consistently formatted. Our config slightly deviates from the default — check out the [rustfmt.toml](https://github.com/squidowl/halloy/blob/main/rustfmt.toml) in the Halloy repo for the current setup.

Before committing, make sure to run: ```cargo +nightly fmt --all```

##### Markdown

Our documentations are written in [markdown](https://rust-lang.github.io/mdBook/format/markdown.html) and built using [mdBook](https://github.com/rust-lang/mdBook). We use a couple of extra pre-processors to improve the output:

- **[mdbook-external-links](https://github.com/jonahgoldwastaken/mdbook-external-links):** Makes external links open in a new tab.
- **[mdbook-linkcheck](https://github.com/Michael-F-Bryan/mdbook-linkcheck):** Verifies that internal links aren’t broken.

You can install both with:

```sh
cargo install mdbook-linkcheck mdbook-external-links
```

##### Other

If your editor supports [EditorConfig](https://editorconfig.org), it will automatically pick up the formatting rules from the [.editorconfig](https://github.com/squidowl/halloy/blob/main/.editorconfig) file in the repo. To double-check everything before committing, you can run [editorconfig-checker](https://github.com/editorconfig-checker/editorconfig-checker):

```sh
cargo install editorconfig-checker
editorconfig-checker
```

### Testing & CI

TODO

### Unit Tests

TODO

### CI with Github Actions

TODO

### Pull-Requests

#### Rebase vs. Merge

TODO

#### Open a PR

TODO

## Contributing documentation

TODO

### Tooling

TODO

### File structure

TODO

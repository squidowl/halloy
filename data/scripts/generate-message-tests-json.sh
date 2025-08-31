#! /usr/bin/env -S bash -e

  script_dir=$(dirname -- "$(realpath -- "${BASH_SOURCE[0]}")")

  cd "$script_dir"/..

  commit_hash=$(git rev-parse HEAD)

  cargo run --features="message_tests" > tests/message/$commit_hash.json

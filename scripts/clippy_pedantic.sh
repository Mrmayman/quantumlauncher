#!/usr/bin/env sh
# cargo clippy -- -W clippy::all  -W clippy::pedantic
cargo clippy -- \
    -W clippy::all  -W clippy::pedantic \
    -A clippy::missing_errors_doc -A clippy::cast_precision_loss \
    -A clippy::cast_sign_loss -A clippy::cast_possible_truncation \
    -A clippy::cast_possible_wrap

#!/bin/sh
cargo --color=always check --tests 2>&1 | less -R --ignore-case

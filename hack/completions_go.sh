#!/bin/sh
# https://carlosbecker.com/posts/golang-completions-cobra/
set -eux
for sh in bash zsh fish; do
    go run $1 completion "$sh" > "$2/$3.$sh"
done

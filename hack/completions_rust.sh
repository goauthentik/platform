#!/bin/sh
# https://carlosbecker.com/posts/golang-completions-cobra/
set -eux
for sh in bash zsh fish; do
    "$1" completions "$sh" > "$2/$3.$sh"
done

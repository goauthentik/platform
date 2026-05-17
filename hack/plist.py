#!/usr/bin/env python3
from plistlib import loads, dumps
from sys import argv


if len(argv) < 3:
    print("""Small helper script to edit plist files, since `Plistbuddy` is macOS only
        Arguments:
          - plist file to edit
          - key to modify
          - value to set""")
    exit()

with open(argv[1], "r+") as _plist:
    data = loads(_plist.read().encode())
    data[argv[2]] = argv[3]
    _plist.seek(0)
    _plist.truncate()
    _plist.write(dumps(data).decode())

#!/bin/bash

set -e

openssl aes-256-cbc -d -in .rpm/private.gpg.enc -out .rpm/private.gpg -k $GPG_KEY
gpg2 --import .rpm/public.gpg
gpg2 --import .rpm/private.gpg
expect -c "spawn gpg2 --edit-key 92D2F72EEDF4105E18C30540E60766D8DA4461A4 trust quit; send \"5\ry\r\"; expect eof"

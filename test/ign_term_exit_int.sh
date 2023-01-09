#! /bin/bash

trap '' SIGTERM
trap 'exit 1' SIGINT

exec sleep infinity
#!/bin/bash

cd "$(dirname "$0")"

idasen-controller --server --server-address=0.0.0.0

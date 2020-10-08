#!/bin/bash

set -e

echo "Release command starting"

/caolo/diesel migration run

echo "Release command finished"

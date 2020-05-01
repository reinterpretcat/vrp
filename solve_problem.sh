#!/bin/bash
# A handy script to run solver on pragmatic problem using routing approximation.
# Output is a solution in pragmatic and geojson formats stored in the same folder
# using the problem's file pattern.

#set -eux

if [[ $# -eq 0 ]] ; then
    echo -e '\033[0;31mExpected a path to file with pragmatic problem definition\033[0m'
    echo "Hint: try 'examples/data/pragmatic/objectives/berlin.default.problem.json'"
    exit 0
fi

ALL_ARGS=("$@")
PROBLEM_FILE_PATH=$1
PROBLEM_FILE_BASE=${PROBLEM_FILE_PATH%.*}
REST_ARGS=("${ALL_ARGS[@]:1}")

cargo run -p vrp-cli --release -- solve pragmatic "${PROBLEM_FILE_PATH}" \
            -o "${PROBLEM_FILE_BASE}_solution.json"                      \
            -g "${PROBLEM_FILE_BASE}_solution.geojson"                   \
            "${REST_ARGS[@]}"
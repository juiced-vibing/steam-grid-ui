#!/usr/bin/env bash
set -euo pipefail

usage() {
	echo "usage: $(basename "$0") [-a|--all] [-h|--help]"
	echo ""
	echo "options:"
	echo "  -a, --all   also remove the rust target folder"
	echo "  -h, --help  show this message"
}

ALL=false

for arg in "$@"; do
	case "$arg" in
	-a | --all) ALL=true ;;
	-h | --help)
		usage
		exit 0
		;;
	*)
		echo "error: unknown option '$arg'"
		usage
		exit 1
		;;
	esac
done

cd "$(dirname "$0")/.."

remove() {
	[[ -d "$1" ]] && echo "$1" && rm -rf "$1"
	return 0
}

remove "packages/app/bin"
remove "packages/app/obj"
remove "dist"
remove "installer/output"

# the rust build is the slow one to get back
if [[ "$ALL" == true ]]; then
	remove "packages/core/target"
fi

echo "clean complete"

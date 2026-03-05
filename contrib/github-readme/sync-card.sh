#!/bin/bash
# TimeForged GitHub Profile Card Sync
# Fetches the SVG activity card from local TimeForged daemon
# and pushes it to your GitHub profile repository.
#
# Requirements:
#   - TF_API_KEY environment variable (or ~/.config/timeforged/config.toml)
#   - Git SSH access to your profile repo
#   - TimeForged daemon running locally
#
# Usage:
#   TF_API_KEY=tf_xxx GITHUB_USER=blysspeak ./sync-card.sh

set -euo pipefail

TF_HOST="${TF_HOST:-http://127.0.0.1:6175}"
TF_API_KEY="${TF_API_KEY:?Set TF_API_KEY environment variable}"
GITHUB_USER="${GITHUB_USER:?Set GITHUB_USER environment variable}"
THEME="${TF_CARD_THEME:-dark}"
DAYS="${TF_CARD_DAYS:-7}"

REPO_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/timeforged/profile-repo"
SVG_FILE="timeforged-card.svg"

# Fetch SVG from local daemon
echo "Fetching card from ${TF_HOST}..."
curl -sf "${TF_HOST}/api/v1/card.svg?key=${TF_API_KEY}&theme=${THEME}&days=${DAYS}" -o "/tmp/${SVG_FILE}"

if [ ! -s "/tmp/${SVG_FILE}" ]; then
    echo "Error: empty or missing SVG" >&2
    exit 1
fi

# Clone or pull profile repo
if [ -d "${REPO_DIR}/.git" ]; then
    git -C "${REPO_DIR}" pull --ff-only --quiet
else
    mkdir -p "$(dirname "${REPO_DIR}")"
    git clone --depth 1 "git@github.com:${GITHUB_USER}/${GITHUB_USER}.git" "${REPO_DIR}"
fi

# Copy SVG and commit if changed
cp "/tmp/${SVG_FILE}" "${REPO_DIR}/${SVG_FILE}"

cd "${REPO_DIR}"
if git diff --quiet "${SVG_FILE}" 2>/dev/null; then
    echo "No changes, skipping."
    exit 0
fi

git add "${SVG_FILE}"
git commit -m "chore: update TimeForged activity card"
git push

echo "Card updated successfully."

#!/bin/bash
set -euxo pipefail

# deploy with rsync and systemd over ssh

source .env set

if [ -z "$SSH_DEPLOY_TARGET" ]; then
    echo "SSH_DEPLOY_TARGET is not set"
    exit 1
fi

echo "Building release binary"
cargo build --release

ssh $SSH_DEPLOY_TARGET -- "mkdir -p /opt/spacetraders_rs"

echo "Deploying to $SSH_DEPLOY_TARGET"
rsync -avzP target/release/run $SSH_DEPLOY_TARGET:/opt/spacetraders_rs
rsync -avzP remote.env $SSH_DEPLOY_TARGET:/opt/spacetraders_rs/.env
rsync -avzP deploy/spacetraders_rs.service $SSH_DEPLOY_TARGET:/etc/systemd/system/spacetraders_rs.service

echo "Restarting service"
ssh $SSH_DEPLOY_TARGET -- "systemctl daemon-reload && systemctl restart spacetraders_rs"
echo "Done"

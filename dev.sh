
source .env set

# proxy to relay, we need the relay because it handles rate limiting
ssh -N $SSH_DEPLOY_TARGET -L 8080:localhost:8080


source .env set

ssh -N $SSH_DEPLOY_TARGET -L 8080:localhost:8080

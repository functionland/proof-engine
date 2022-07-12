
#!/usr/bin/env bash

docker build \
    --build-arg USER_UID=$(id -u ${USER}) \
    --build-arg USER_GID=$(id -g ${USER}) \
    -t fula-funge:dev .

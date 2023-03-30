FROM rust:1.62 AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        libudev-dev \
        libasound2-dev \
        libgtk-3-dev \
        libatk1.0-dev \
        libglib2.0-dev

WORKDIR /proof-engine
COPY . /proof-engine
RUN cargo build --features headless --locked --release

FROM debian:bullseye

RUN apt-get update && apt-get install -y --no-install-recommends \
        libasound2

ARG USERNAME=fula
ARG USER_UID=1000
ARG USER_GID=$USER_UID

# Non-root user
RUN groupadd --gid $USER_GID $USERNAME \
    && useradd --uid $USER_UID --gid $USER_GID -m $USERNAME \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /proof-engine/target/release/proof-engine /home/$USERNAME/

RUN chown ${USER_UID}:${USER_GID} /home/$USERNAME/proof-engine

USER $USERNAME
ENV HOME /home/$USERNAME
WORKDIR $HOME

ENTRYPOINT ["./proof-engine"]
CMD ["//Alice", "--pool-id", "1000000", "--asset-id", "1000000"]

FROM rust:1.62 AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        libudev-dev \
        libasound2-dev \
        libgtk-3-dev \
        libatk1.0-dev \
        libglib2.0-dev

WORKDIR /fula-funge
COPY . /fula-funge
RUN cargo build --features headless --locked --release

FROM ubuntu:22.04

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

COPY ./assets/sensors /home/$USERNAME/assets/sensors
COPY --from=builder /fula-funge/target/release/fula-funge /home/$USERNAME/

RUN chown ${USER_UID}:${USER_GID} /home/$USERNAME/assets
RUN chown ${USER_UID}:${USER_GID} /home/$USERNAME/fula-funge

USER $USERNAME
ENV HOME /home/$USERNAME
WORKDIR $HOME

ENTRYPOINT ["./fula-funge"]
CMD ["//Alice", "--pool-id", "1000000"]

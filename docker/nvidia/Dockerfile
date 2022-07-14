FROM paritytech/ci-linux:production AS builder
WORKDIR /proof-engine
COPY . /proof-engine

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        libudev-dev \
        libasound2-dev \
        libgtk-3-dev \
        libatk1.0-dev \
        libglib2.0-dev

RUN cargo build --locked --release

FROM nvidia/cudagl:11.4.2-devel as base
ENV NVIDIA_DRIVER_CAPABILITIES ${NVIDIA_DRIVER_CAPABILITIES},display

ENV DEBIAN_FRONTEND=noninteractive
ENV LANG C.UTF-8
ENV LC_ALL C.UTF-8

RUN apt-get purge -y ".*:i386" && dpkg --remove-architecture i386

RUN apt-get update && apt-get install -y --no-install-recommends \
        software-properties-common \
        libnss3 \
        libasound2 \
        libxcursor1 \
        libxrandr2 \
        libxi6

# Enable Vulkan support
RUN add-apt-repository ppa:graphics-drivers/ppa -y && apt-get install -y --no-install-recommends \
    libvulkan1 mesa-vulkan-drivers vulkan-utils glslang-tools

# Set Coordinated Universal Time
RUN ln -sf /usr/share/zoneinfo/UTC /etc/localtime
RUN apt-get update && apt-get install -y tzdata

ARG USERNAME=sim
ARG USER_UID=1000
ARG USER_GID=$USER_UID

# Non-root user
RUN groupadd --gid $USER_GID $USERNAME \
    && useradd --uid $USER_UID --gid $USER_GID -m $USERNAME \
    && mkdir -p /home/$USERNAME/.vscode-server /home/$USERNAME/.vscode-server-insiders \
    && chown ${USER_UID}:${USER_GID} /home/$USERNAME/.vscode-server* \
    && apt-get install -y sudo \
    && echo $USERNAME ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/$USERNAME \
    && usermod -a -G audio,video $USERNAME \
    && chmod 0440 /etc/sudoers.d/$USERNAME \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

# Copy the proof-engine executable and assets

COPY --from=builder /proof-engine/target/release/proof-engine /home/$USERNAME/
COPY ./assets /home/$USERNAME/assets

RUN chown ${USER_UID}:${USER_GID} /home/$USERNAME/assets
RUN chown ${USER_UID}:${USER_GID} /home/$USERNAME/proof-engine

USER $USERNAME
ENV HOME /home/$USERNAME
WORKDIR $HOME

# Switch back to dialog for any ad-hoc use of apt-get
ENV DEBIAN_FRONTEND=

ENTRYPOINT ["./proof-engine"]

CMD ["//Alice", "--pool-id", "1000000"]

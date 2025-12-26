FROM rust:1.92-bookworm

# Install system dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    iproute2 \
    iputils-ping \
    netcat-traditional \
    libpcap-dev \
    net-tools \
    vim \
    git \
    sudo \
    tcpdump \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install just

WORKDIR /workspace

# Enhance the shell experience
RUN echo 'export PS1="\[\033[01;32m\]\u@\h\[\033[00m\]:\[\033[01;34m\]\w\[\033[00m\]\n\$ "' >> /root/.bashrc && \
    echo 'export PROMPT_COMMAND="echo"' >> /root/.bashrc

CMD ["/bin/bash"]

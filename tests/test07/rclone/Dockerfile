FROM fedora:42

ARG USERNAME=default_usr


RUN sed -i '/tsflags=nodocs/s/^/#/' /etc/dnf/dnf.conf
RUN dnf install -y cronie par2cmdline rclone coreutils man-pages man-db zsh tar gzip  dialog iputils \
    vim passwd \
     jq time tini openssh-clients azure-cli rsync sqlite git sqlite-devel curl glibc libicu

# powershell
# Download and install the latest stable PowerShell release
RUN LATEST_VERSION=$(curl -s https://api.github.com/repos/PowerShell/PowerShell/releases/latest | jq -r '.tag_name') && \
    # auto-detect ARCH
    UNAME_M=$(uname -m) && \
    if [ "$UNAME_M" = "x86_64" ]; then ARCH=linux-x64; \
    elif [ "$UNAME_M" = "aarch64" ]; then ARCH=linux-arm64; \
    else echo "Unsupported architecture: $UNAME_M" >&2; exit 1; fi && \
    curl -L -o /tmp/powershell.tar.gz "https://github.com/PowerShell/PowerShell/releases/download/${LATEST_VERSION}/powershell-${LATEST_VERSION#v}-${ARCH}.tar.gz" && \
    mkdir -p /opt/microsoft/powershell/${LATEST_VERSION} && \
    tar -xzf /tmp/powershell.tar.gz -C /opt/microsoft/powershell/${LATEST_VERSION} && \
    ln -s /opt/microsoft/powershell/${LATEST_VERSION}/pwsh /usr/bin/pwsh && \
    rm -f /tmp/powershell.tar.gz

USER root
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["crond", "-f"]

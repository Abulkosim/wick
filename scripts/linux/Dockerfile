# A Linux desktop in a container for working on Wick from macOS or Windows: Ubuntu 22.04 (the
# release build's base), the Tauri toolchain, systemd with logind and polkit, D-Bus, Xvfb and an
# Xfce panel that hosts StatusNotifier tray icons. See CONTRIBUTING.md → Linux.
FROM ubuntu:26.04@sha256:6a06111c3b0a8600feadb270aec771dc239bf407b25625f93f44ab00e11f4d3c
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential curl wget file ca-certificates pkg-config libssl-dev git \
    libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev patchelf \
    systemd systemd-sysv dbus dbus-x11 dbus-user-session policykit-1 \
    xvfb x11-utils xdotool wmctrl imagemagick \
    xfce4-panel xfwm4 xfce4-statusnotifier-plugin xfce4-settings xfce4-power-manager xterm \
    fonts-dejavu-core fonts-noto-color-emoji xdg-desktop-portal xdg-desktop-portal-gtk \
    && rm -rf /var/lib/apt/lists/*
RUN curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain 1.99.1 --component clippy,rustfmt
ENV PATH=/root/.cargo/bin:$PATH
STOPSIGNAL SIGRTMIN+3
CMD ["/lib/systemd/systemd"]

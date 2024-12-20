# Install gled

## Linux (RPM)

```bash
sudo tee -a /etc/yum.repos.d/gled.repo << 'EOF'
[gitlab.com_gled_repo]
name=gitlab.com_gled_repo
baseurl=https://photonenkollektiv.gitlab.io/gled2/rpm/
enabled=1
gpgcheck=1
repo_gpgcheck=1
gpgkey=https://gitlab.com/photonenkollektiv/gled2/-/raw/main/.rpm/public.gpg
metadata_expire=1h
EOF
```

```
sudo dnf install gled
```

## Mac/Windows

Download the latest release from the [releases page](https://gitlab.com/photonenkollektiv/gled2/-/releases).

## Other

Install with cargo:

```
cargo install --git https://gitlab.com/photonenkollektiv/gled2
```

# Source code:

[https://gitlab.com/photonenkollektiv/gled2](https://gitlab.com/photonenkollektiv/gled2)

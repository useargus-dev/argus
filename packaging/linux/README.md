# Linux packaging extras

## Polkit (optional)

For GUI-friendly elevation instead of a terminal `sudo` password prompt, install the policy file:

```bash
sudo install -m 644 org.argus.redirector.policy \
  /usr/share/polkit-1/actions/org.argus.redirector.policy
```

Adjust the `exec.path` annotation if Argus is installed outside `/opt/Argus`.

`argus run` still uses `sudo` by default (mitmproxy transparent mode). Polkit integration for `pkexec` can be added in a future release; the policy documents the intended action ID for distro packages.

# WinDivert runtime (Windows)

`WinDivert.dll` and `WinDivert64.sys` are **not committed** to git. They are staged here before release builds:

```powershell
cargo build --release -p argus-redirector-windows
./scripts/stage-windivert.ps1
```

The redirector and installer bundle expect these files next to `argus-redirector-windows.exe` under `lib/argus/`.

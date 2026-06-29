# WinDivert runtime (Windows)

`WinDivert.dll` and `WinDivert64.sys` are **not committed** to git. Fetch them before release builds:

```powershell
./scripts/fetch-windivert.ps1
cargo build --release -p argus-redirector-windows
./scripts/stage-windivert.ps1
```

CI runs `fetch-windivert.ps1` automatically and sets `WINDIVERT_PATH` to this directory.

The redirector and installer bundle expect these files next to `argus-redirector-windows.exe` under `lib/argus/`.

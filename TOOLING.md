# EyeRest Tooling Notes

This project is intended to run on Windows with Tauri, Rust, Node.js, and npm.

## npm

In PowerShell, `npm` may be blocked by script execution policy because it resolves to `npm.ps1`.

Use:

```powershell
npm.cmd --version
npm.cmd install
npm.cmd run tauri dev
```

From Command Prompt, plain `npm` should also work:

```cmd
npm --version
npm install
npm run tauri dev
```

## Node.js

Check Node.js with:

```powershell
node --version
```

## Rust

If Rust is on `PATH`, use:

```powershell
rustc --version
cargo --version
rustup show active-toolchain
rustup target list --installed
```

If `rustc`, `cargo`, or `rustup` are not found, restart the terminal after installing Rust and confirm that `%USERPROFILE%\.cargo\bin` is on `PATH`.

The expected toolchain is:

```text
stable-x86_64-pc-windows-msvc
```

The expected installed target is:

```text
x86_64-pc-windows-msvc
```

## Visual Studio Build Tools

The MSVC compiler is normally available inside the Visual Studio developer environment:

```powershell
& cmd /c 'call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" -arch=x64 -host_arch=x64 >nul && cl'
```

## WebView2

Tauri requires WebView2 on Windows. Check the installed runtime folder:

```powershell
Get-ChildItem -Path 'C:\Program Files (x86)\Microsoft\EdgeWebView\Application' -Directory
```

## Common Development Commands

After the project files are filled in and dependencies are installed:

```powershell
npm.cmd install
npm.cmd run tauri dev
npm.cmd run tauri build
```

Cargo must be available on `PATH` for the packaging script and the GitHub Actions workflow.

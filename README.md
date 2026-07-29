<h1>
  EyeRest
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows-0078D4">
  <img alt="License" src="https://img.shields.io/badge/license-MIT-green">
</h1>

<img src="/imgs/icons/eyerest-icon.svg" width="128" height="128" align="right" alt="EyeRest logo"/>

> The healthy screen-break reminder for Windows

EyeRest is a lightweight Windows reminder that helps you follow the 20-20-20 rule.

<br>

## Features

- Configurable work and rest intervals
- Fullscreen rest overlay on selected monitors
- System tray controls
- Idle and fullscreen-app suppression
- Application whitelist
- Optional start with Windows
- Installed, standalone, and portable distributions

## Screenshots

<p align="center">
  <img src="/imgs/setting_1.png" alt="EyeRest setting interface" width="49%">
  <img src="/imgs/setting_2.png" alt="EyeRest setting interface" width="49%">
</p>

<p align="center">
  <img src="/imgs/rest_screen.png" alt="EyeRest rest screen" width="70%">
</p>

## Release

Each release may include several Windows packages. Choose the package that best matches how you want to use EyeRest.

### `EyeRest-standalone.exe`

Runs EyeRest directly without installation.

- No installation is required.
- You can place the executable in any folder and run it.
- Settings and logs are stored in your Windows AppData folder.
- This version does not keep its settings beside the executable.

### `EyeRest-portable-windows-x64.zip`

A fully portable package that stores its data beside the application.

- Extract the ZIP archive before running EyeRest.
- Run `EyeRest.exe` from the extracted folder.
- Keep the `EyeRest.portable` marker beside `EyeRest.exe`.
- Settings are stored in the package's `data` folder.
- Logs are stored in the package's `logs` folder.
- The extracted folder can be moved between locations or compatible Windows computers.

Do not run the application directly from inside the ZIP archive. Extract the entire archive first so that all required files remain together.

### `EyeRest_x.y.z_x64-setup.exe`

The standard Windows installer.

- Recommended for most users.
- Installs EyeRest through a guided setup process.
- Can create application shortcuts and register an uninstaller.
- Appears in the Windows installed-applications list.

The `x.y.z` portion of the filename represents the release version.

### `EyeRest_x.y.z_x64_en-US.msi`

The Windows Installer package.

- Provides the same installed version of EyeRest in MSI format.
- Suitable for managed deployment, administrative installation, and environments that prefer MSI packages.
- Can be installed through the Windows Installer interface or supported deployment tools.

The `x.y.z` portion of the filename represents the release version.

### Which One Should I Use?

- Choose `EyeRest_x.y.z_x64-setup.exe` for a normal Windows installation.
- Choose `EyeRest-standalone.exe` to run EyeRest without installing it while keeping settings in AppData.
- Choose `EyeRest-portable-windows-x64.zip` to keep the application, settings, and logs together in one folder.
- Choose `EyeRest_x.y.z_x64_en-US.msi` for MSI-based installation or managed deployment.



## Development

### Requirements

To build EyeRest, install:

- [Node.js 20 or later](https://nodejs.org/)
- [Rust stable with the MSVC toolchain](https://www.rust-lang.org/tools/install)
- [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- [WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/)

### Getting Started

Clone the repository and install the locked JavaScript dependencies:

```powershell
git clone https://github.com/makercyf/EyeRest.git
Set-Location EyeRest
npm ci
```

Start the application in development mode:

```powershell
npm run dev
```

Run the Rust checks and tests:

```powershell
npm run check
npm test
```

## Build Packages

Build the NSIS setup executable and MSI installer:

```powershell
npm run build:installed
```

The installers are written under:

```text
src-tauri/target/release/bundle/nsis/
src-tauri/target/release/bundle/msi/
```

Create the portable application:

```powershell
./scripts/package-portable.ps1
```

The portable package is written to `dist/portable/EyeRest`. The `EyeRest.portable` marker beside `EyeRest.exe` enables portable mode. Settings are then stored in `data/settings.json`, and logs are stored in `logs/`.

Keep `scripts/package-portable.ps1` in the repository. It documents the portable layout, makes local packages reproducible, and is also used by continuous integration. If the release binary has already been built, use:

```powershell
./scripts/package-portable.ps1 -SkipBuild
```

## Security Disclaimer

The executable files and installers distributed by this project are compiled in the GitHub Actions build environment using the source code and build configuration contained in this repository. The build workflow:

1. Installs the locked npm and Cargo dependencies.
2. Runs the Rust checks and tests.
3. Builds the Windows NSIS and MSI installers.
4. Creates the standalone executable, portable package, and ZIP archive.
5. Calculates SHA-256 checksums for the distributed executable files, archives, and installers.
6. Uploads the generated packages and `SHA256SUMS.txt` as release artifacts.

SHA-256 checksums are provided so that you can verify that a downloaded file has not been modified or corrupted after it was built. A matching checksum confirms file integrity, but it does not guarantee that a file is secure, free from vulnerabilities, or suitable for your environment.

To verify a downloaded file in PowerShell:

```powershell
Get-FileHash -Algorithm SHA256 -LiteralPath ./EyeRest-standalone.exe
```

Compare the displayed hash with the corresponding entry in `SHA256SUMS.txt`.

You should not rely solely on checksums, antivirus results, or the fact that a file was built through GitHub Actions. Before running software obtained from another person or project, you are strongly encouraged to review the source code, dependencies, build scripts, and workflow configuration.

If you have security or malware concerns, you can:

- Review the source code and build scripts yourself.
- Use static-analysis tools or an AI-assisted coding agent to help audit the code.
- Review the GitHub Actions workflow and its build logs.
- Scan the downloaded files with reputable security tools.
- Compile the application directly from the reviewed source code by following the build instructions in this README.

Automated or AI-assisted audits can help identify potential issues, but they should not be treated as a substitute for independent security review or professional judgment.

By downloading, building, or running EyeRest, you accept responsibility for evaluating whether the software is appropriate for your system and security requirements.

## Privacy

EyeRest stores its settings and diagnostic logs locally. It does not require an account or cloud service.

The storage location depends on the selected release package:

- The installed and standalone versions store application data in the user's Windows AppData folder.
- The portable version stores settings and logs beside the application in its extracted folder.

## Contributing

Issues and pull requests are welcome. Before submitting a change, run:

```powershell
npm ci
npm run check
npm test
```

## License

This project is licensed under the MIT License.

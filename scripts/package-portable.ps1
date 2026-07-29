param(
  [switch]$SkipBuild,
  [string]$OutputDirectory
)

$ErrorActionPreference = "Stop"
$ProjectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$TargetDir = Join-Path $ProjectRoot "src-tauri\target\release"
$ExePath = Join-Path $TargetDir "eyerest.exe"
$PortableDir = if ($OutputDirectory) {
  if ([System.IO.Path]::IsPathRooted($OutputDirectory)) {
    [System.IO.Path]::GetFullPath($OutputDirectory)
  } else {
    [System.IO.Path]::GetFullPath((Join-Path $ProjectRoot $OutputDirectory))
  }
} else {
  Join-Path $ProjectRoot "dist\portable\EyeRest"
}
$BundleIconPath = Join-Path $ProjectRoot "src-tauri\icons\icon.ico"

if (-not (Test-Path -LiteralPath $BundleIconPath -PathType Leaf)) {
  throw "Application icon not found at '$BundleIconPath'."
}

if (-not $SkipBuild) {
  $Cargo = Get-Command cargo -ErrorAction Stop
  & $Cargo.Source build `
    --locked `
    --release `
    --manifest-path (Join-Path $ProjectRoot "src-tauri\Cargo.toml")
}

if (-not (Test-Path -LiteralPath $ExePath -PathType Leaf)) {
  throw "Release binary not found at '$ExePath'. Build the application before packaging."
}

if (Test-Path -LiteralPath $PortableDir) {
  Remove-Item -LiteralPath $PortableDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $PortableDir | Out-Null
Copy-Item -LiteralPath $ExePath -Destination (Join-Path $PortableDir "EyeRest.exe") -Force
New-Item -ItemType File -Force -Path (Join-Path $PortableDir "EyeRest.portable") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $PortableDir "data") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $PortableDir "logs") | Out-Null

Write-Host "Portable package created at $PortableDir"

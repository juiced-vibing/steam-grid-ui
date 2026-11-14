param(
  [Parameter(Mandatory)]
  [string]$Version
)

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path "$PSScriptRoot\..").Path
$Dist = Join-Path $Root "dist"

# winget installs inno setup without putting it on PATH
$Iscc = @("iscc", "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe", "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe") |
  Where-Object { Get-Command $_ -ErrorAction SilentlyContinue } |
  Select-Object -First 1
if (-not $Iscc) { throw "Inno Setup not found: winget install JRSoftware.InnoSetup" }

if (-not (Test-Path (Join-Path $Dist "SteamGridUI.exe"))) {
  dotnet publish (Join-Path $Root "packages\app\SteamGridUI.csproj") -c Release "-p:Version=$Version" -o $Dist
  if ($LASTEXITCODE -ne 0) { exit 1 }
}

& $Iscc "/DAppVersion=$Version" "/DExeDir=$Dist" (Join-Path $Root "installer\SteamGridUI.iss")
if ($LASTEXITCODE -ne 0) { exit 1 }

Copy-Item -Force (Join-Path $Root "installer\output\SteamGridUI-$Version-setup.exe") $Dist

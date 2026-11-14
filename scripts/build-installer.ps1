param(
  [Parameter(Mandatory)]
  [string]$Version
)

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path "$PSScriptRoot\..").Path
$Dist = Join-Path $Root "dist"

if (-not (Test-Path (Join-Path $Dist "SteamGridUI.exe"))) {
  dotnet publish (Join-Path $Root "packages\app\SteamGridUI.csproj") -c Release "-p:Version=$Version" -o $Dist
  if ($LASTEXITCODE -ne 0) { exit 1 }
}

iscc "/DAppVersion=$Version" "/DExeDir=$Dist" (Join-Path $Root "installer\SteamGridUI.iss")
if ($LASTEXITCODE -ne 0) { exit 1 }

Copy-Item -Force (Join-Path $Root "installer\output\SteamGridUI-$Version-setup.exe") $Dist

[CmdletBinding(SupportsShouldProcess)]
param(
  [switch] $All
)

$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

function Remove-Directory {
  [CmdletBinding(SupportsShouldProcess)]
  param([string] $Path)

  if ((Test-Path $Path) -and $PSCmdlet.ShouldProcess($Path, "remove")) {
    Write-Output $Path
    Remove-Item -Recurse -Force $Path
  }
}

foreach ($path in "packages\app\bin", "packages\app\obj", "dist", "installer\output") {
  Remove-Directory $path
}

# the rust build is the slow one to get back
if ($All) {
  Remove-Directory "packages\core\target"
}

Write-Output "clean complete"

<#
.SYNOPSIS
  Print the version the next release gets when none is given.
.DESCRIPTION
  The first two numbers are the workspace version in Cargo.toml, which follows upstream. The last one is
  one more than the highest release of that major.minor already published in the repository, so the
  numbering continues whatever the release workflow is called and however often it ran.
  Needs gh with a token (GH_TOKEN), the repository (GITHUB_REPOSITORY, or -Repository owner/name) and the
  app's tag prefix (-TagPrefix editor-).
#>
param([string] $Repository = $env:GITHUB_REPOSITORY, [string] $TagPrefix = '')
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if (-not $Repository) { throw 'Give -Repository owner/name, or set GITHUB_REPOSITORY.' }
$manifest = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot '../../Cargo.toml')
if ($manifest -notmatch '(?ms)^\[workspace\.package\].*?^version\s*=\s*"(\d+)\.(\d+)\.') {
  throw "Can't read the workspace version in Cargo.toml."
}
$major = $Matches[1]
$minor = $Matches[2]
# The repository holds more than one app: each app's tags start with its own prefix (editor-v1.2.3).
$prefix = "${TagPrefix}v${major}.${minor}."
$tags = @(gh release list --repo $Repository --limit 1000 --json tagName --jq '.[].tagName')
if ($LASTEXITCODE -ne 0) { throw "gh release list failed for $Repository." }
$last = 0
foreach ($tag in $tags) {
  $name = "$tag".Trim()
  if (-not $name.StartsWith($prefix)) { continue }
  $rest = $name.Substring($prefix.Length)
  # Windows Installer's third version number stops at 65535.
  if ($rest -match '^\d{1,5}$') { $last = [Math]::Max($last, [int]$rest) }
}
"${major}.${minor}.$($last + 1)"

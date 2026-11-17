<#
.SYNOPSIS
  Reads the brand files (ntm.env, pixel.env) for the installer scripts and the workflows.
.DESCRIPTION
  Dot-source it:  . (Join-Path $Root '..\brand\brand.ps1')
  Get-Brand [-Id ntm|pixel]   the brand's keys as an ordered hashtable ($b.EDITOR_EXE, $b.COMPANY, ...),
                              plus EDITOR_TAG_PREFIX and PHOTO_TAG_PREFIX. -Id defaults to $env:NTM_BRAND,
                              then to ntm. The Rust side (brand/build.rs) reads the same files.
  Get-BrandIds                the ids of all brand files
  ConvertTo-Brand             a text with the default brand's names swapped for a brand's
  Get-WixDefines              the -d arguments that hand an app's names and GUIDs to `wix build`
  Read-BrandedJson            a JSON file whose @PRODUCT@ @EXE@ @FOLDER@ @UPGRADE_CODE@ @VENDOR_DIR@ are filled in
#>
# (No Set-StrictMode here: dot-sourcing would switch it on in the script that loads this file.)
$script:BrandDir = $PSScriptRoot

function Get-BrandIds {
  Set-StrictMode -Version Latest
  @(Get-ChildItem -LiteralPath $script:BrandDir -Filter '*.env' | ForEach-Object { $_.BaseName } | Sort-Object)
}

function Get-Brand {
  param([string] $Id)
  Set-StrictMode -Version Latest
  if (-not $Id) { $Id = $env:NTM_BRAND }
  if (-not $Id) { $Id = 'ntm' }
  $Id = $Id.Trim().ToLowerInvariant()
  if ($Id -notmatch '^[a-z0-9]+$') { throw "A brand id is lower-case letters and digits, not '$Id'." }
  $path = Join-Path $script:BrandDir "$Id.env"
  if (-not (Test-Path -LiteralPath $path)) { throw "There is no brand '$Id' (brand files: $((Get-BrandIds) -join ', '))." }
  $brand = [ordered]@{}
  foreach ($line in Get-Content -LiteralPath $path -Encoding utf8) {
    $text = $line.Trim()
    if ($text -eq '' -or $text.StartsWith('#')) { continue }
    $at = $text.IndexOf('=')
    if ($at -lt 1) { throw "${path}: expected KEY=value, got '$text'." }
    $brand[$text.Substring(0, $at).Trim()] = $text.Substring($at + 1).Trim()
  }
  if ($brand['ID'] -ne $Id) { throw "${path}: ID is '$($brand['ID'])', expected '$Id'." }
  # A release tag is <app>-<TAG_INFIX>v<version>.
  $brand['EDITOR_TAG_PREFIX'] = "editor-$($brand['TAG_INFIX'])"
  $brand['PHOTO_TAG_PREFIX'] = "photo-$($brand['TAG_INFIX'])"
  $brand
}

# The sources and the files shipped with the apps (NOTICE, README) spell the default brand's names.
function ConvertTo-Brand {
  param([Parameter(Mandatory = $true)] [AllowEmptyString()] [string] $Text, [Parameter(Mandatory = $true)] $Brand)
  Set-StrictMode -Version Latest
  $ntm = Get-Brand 'ntm'
  # Longest names first: the product names contain the company's.
  foreach ($key in 'EDITOR_PRODUCT', 'PHOTO_PRODUCT', 'COMPANY', 'VENDOR_DIR', 'SLUG') {
    if ($ntm[$key] -ne $Brand[$key]) { $Text = $Text.Replace($ntm[$key], $Brand[$key]) }
  }
  $Text
}

# The `wix build` arguments that name one app of a brand: the .wxs files take them as $(var.<Name>).
function Get-WixDefines {
  param([Parameter(Mandatory = $true)] $Brand, [Parameter(Mandatory = $true)] [ValidateSet('EDITOR', 'PHOTO')] [string] $App)
  Set-StrictMode -Version Latest
  $exe = $Brand["${App}_EXE"]
  @(
    '-d', "ProductName=$($Brand["${App}_PRODUCT"])",
    '-d', "Manufacturer=$($Brand['COMPANY'])",
    '-d', "UpgradeCode=$($Brand["${App}_UPGRADE_CODE"])",
    '-d', "ComponentGuid=$($Brand["${App}_COMPONENT_GUID"])",
    '-d', "SuiteFolder=$($Brand['VENDOR_DIR'])",
    '-d', "AppFolder=$($Brand["${App}_FOLDER"])",
    '-d', "ExeName=$exe.exe",
    '-d', "CliExeName=$exe-cli.exe"
  )
}

# install-policy.json and friends: the file with the brand's values put in for its @TOKENS@, as an object.
function Read-BrandedJson {
  param([Parameter(Mandatory = $true)] [string] $Path, [Parameter(Mandatory = $true)] $Brand, [Parameter(Mandatory = $true)] [ValidateSet('EDITOR', 'PHOTO')] [string] $App)
  Set-StrictMode -Version Latest
  $text = Get-Content -Raw -LiteralPath $Path -Encoding utf8
  $values = [ordered]@{
    '@PRODUCT@'      = $Brand["${App}_PRODUCT"]
    '@EXE@'          = $Brand["${App}_EXE"]
    '@FOLDER@'       = $Brand["${App}_FOLDER"]
    '@UPGRADE_CODE@' = $Brand["${App}_UPGRADE_CODE"]
    '@VENDOR_DIR@'   = $Brand['VENDOR_DIR']
  }
  foreach ($token in $values.Keys) { $text = $text.Replace($token, $values[$token]) }
  if ($text -match '@[A-Z_]+@') { throw "${Path}: no value for $($Matches[0])." }
  $text | ConvertFrom-Json
}

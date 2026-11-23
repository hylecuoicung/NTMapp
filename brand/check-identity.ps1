<#
.SYNOPSIS
  Check that an installer still has the identity of the newest release of the same app and brand.
.DESCRIPTION
  What makes an installer the same product as the one people have: the UpgradeCode, the component
  GUIDs, the install folders, the registry keys it writes, the file names, the shortcuts. If any of
  them changes, the new installer no longer updates the installed copy: it installs beside it. This
  compares the tables of the new MSI with those of the newest published release of the brand's app
  (editor-v0.20.2 for the NTM Digital Editor) and fails on any difference, except rows the new
  installer only adds to the file-type and registry tables (a newly registered file type). A brand
  with no release yet has nothing to compare with, and passes.

  Needs Windows (the Windows Installer COM object) and gh with a token (GH_TOKEN) for the download.
  Run by the test-build workflows; it installs nothing and writes only to a temporary folder.
.EXAMPLE
  pwsh brand/check-identity.ps1 -App editor -Msi editor/dist/release/ntmdigital-editor-0.20.3-windows-x64.msi
#>
param(
  [Parameter(Mandatory = $true)] [ValidateSet('editor', 'photo')] [string] $App,
  [Parameter(Mandatory = $true)] [string] $Msi,
  [string] $Brand = $env:NTM_BRAND,
  [string] $Repository = $env:GITHUB_REPOSITORY
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'brand.ps1')
$b = Get-Brand $Brand
if (-not $Repository) { $Repository = $b['REPO'] }

# ---- the newest release of this app under this brand
$prefix = $b["$($App.ToUpper())_TAG_PREFIX"] + 'v'
$tags = @(gh release list --repo $Repository --limit 200 --json tagName --jq '.[].tagName')
if ($LASTEXITCODE -ne 0) { throw "gh release list failed for $Repository." }
$found = @($tags | ForEach-Object { "$_".Trim() } |
  Where-Object { $_.StartsWith($prefix) -and $_.Substring($prefix.Length) -match '^\d+\.\d+\.\d+$' } |
  Sort-Object { [version]$_.Substring($prefix.Length) } -Descending)
if (-not $found) {
  Write-Output "PASS: there is no earlier $prefix* release of $($b['ID']), nothing to compare with."
  return
}
$tag = $found[0]
$temp = Join-Path ([IO.Path]::GetTempPath()) "identity-$PID"
New-Item -ItemType Directory -Force -Path $temp | Out-Null
try {
  gh release download $tag --repo $Repository --pattern '*.msi' --dir $temp
  if ($LASTEXITCODE -ne 0) { throw "gh release download $tag failed." }
  $old = @(Get-ChildItem -LiteralPath $temp -Filter '*.msi')
  if ($old.Count -ne 1) { throw "expected one .msi in $tag, found $($old.Count)." }

  function Read-Rows($Database, [string] $Sql, [int] $Columns) {
    $view = $Database.OpenView($Sql)
    try {
      [void]$view.Execute()
      while ($null -ne ($record = $view.Fetch())) {
        try {
          $values = @(for ($i = 1; $i -le $Columns; $i++) {
            $record.GetType().InvokeMember('StringData', 'GetProperty', $null, $record, @($i))
          })
          , ($values -join ' | ')
        } finally { [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($record) }
      }
    } finally {
      [void]$view.Close()
      [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($view)
    }
  }

  # (table, columns, filter): what is read from each. File names are compared by their long form.
  $queries = [ordered]@{
    'Property'  = @('SELECT `Property`, `Value` FROM `Property` WHERE `Property`=''UpgradeCode'' OR `Property`=''ProductName'' OR `Property`=''Manufacturer'' OR `Property`=''ALLUSERS'' OR `Property`=''MSIINSTALLPERUSER''', 2)
    'Component' = @('SELECT `Component`, `ComponentId`, `Directory_` FROM `Component`', 3)
    'Directory' = @('SELECT `Directory`, `Directory_Parent`, `DefaultDir` FROM `Directory`', 3)
    'Registry'  = @('SELECT `Root`, `Key`, `Name`, `Value` FROM `Registry`', 4)
    'File'      = @('SELECT `File`, `Component_`, `FileName` FROM `File`', 3)
    'Shortcut'  = @('SELECT `Shortcut`, `Directory_`, `Name`, `Component_` FROM `Shortcut`', 4)
    'ProgId'    = @('SELECT `ProgId`, `ProgId_Parent`, `Description` FROM `ProgId`', 3)
    'Extension' = @('SELECT `Extension`, `Component_`, `ProgId_` FROM `Extension`', 3)
  }

  function Get-Identity([string] $Path) {
    $installer = New-Object -ComObject WindowsInstaller.Installer
    $database = $installer.OpenDatabase((Resolve-Path -LiteralPath $Path).Path, 0)
    $out = [ordered]@{}
    try {
      foreach ($name in $queries.Keys) {
        $sql, $cols = $queries[$name]
        # a table the package does not have (no ProgId rows in the editor) reads as empty
        $rows = try { @(Read-Rows $database $sql $cols) } catch { @() }
        $out[$name] = @($rows | Sort-Object)
      }
    } finally {
      [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($database)
      [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($installer)
    }
    $out
  }

  # The short half of a file, folder or shortcut name is generated: only the long name is its name.
  function Format-Row([string] $Table, [string] $Row) {
    if ($Table -notin 'File', 'Directory', 'Shortcut') { return $Row }
    $parts = $Row -split ' \| '
    $parts[2] = ($parts[2] -split '\|')[-1]
    $parts -join ' | '
  }

  $before = Get-Identity $old[0].FullName
  $after = Get-Identity $Msi
  $problems = New-Object System.Collections.Generic.List[string]
  $mayAdd = @('ProgId', 'Extension', 'Registry')
  foreach ($name in $queries.Keys) {
    $a = @($before[$name] | ForEach-Object { Format-Row $name $_ })
    $z = @($after[$name] | ForEach-Object { Format-Row $name $_ })
    foreach ($row in @($a | Where-Object { $_ -notin $z })) { $problems.Add("${name}: $tag has [$row], the new installer does not") }
    foreach ($row in @($z | Where-Object { $_ -notin $a })) {
      # A file type registered from now on (the editor's .fcproj) adds rows in the component that was
      # already there, and the upgrade removes the old copy before installing, so it is still the same
      # product. A changed row is also a removed one, and that still fails.
      if ($name -in $mayAdd) { Write-Output "Added ${name}: [$row]"; continue }
      $problems.Add("${name}: the new installer has [$row], $tag does not")
    }
  }
  if ($problems.Count) {
    $problems | ForEach-Object { Write-Output "::error::$_" }
    throw "The installer is no longer the same product as $tag ($($problems.Count) differences): installed copies would not be updated by it."
  }
  Write-Output "PASS: the installer has the identity of $tag ($(($queries.Keys | ForEach-Object { "$_ $($after[$_].Count)" }) -join ', '))."
} finally {
  Remove-Item -Recurse -Force $temp -ErrorAction SilentlyContinue
}

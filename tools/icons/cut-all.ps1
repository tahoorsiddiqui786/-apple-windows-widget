param([string]$Icons, [string]$OutDir, [int]$Size = 512)
# Cuts glass icon tiles out of flat white/black JPEG backgrounds (e.g. images
# saved from WhatsApp/ChatGPT) into 512x512 transparent PNGs.
#
#   powershell -ExecutionPolicy Bypass -File tools\icons\cut-all.ps1 `
#     -Icons assets\icons\originals -OutDir assets\icons
#
# Reads every app-*.jpg in -Icons (never modified) and writes app-*.png to
# -OutDir (re-running overwrites those PNGs). Icons named *-clay use a
# sensitive edge threshold and rounder corners for white-on-white tiles.
# Always eyeball the results: unusual tiles may need their own settings.
Add-Type -ReferencedAssemblies System.Drawing -Path (Join-Path $PSScriptRoot "IconCut.cs")
New-Item -ItemType Directory -Force $OutDir | Out-Null
$sw = [Diagnostics.Stopwatch]::StartNew()
$files = Get-ChildItem $Icons -File -Filter "app-*.jpg" | Sort-Object Name
foreach ($f in $files) {
  $clay = $f.BaseName -like '*-clay'
  $thr = if ($clay) { 6 } else { 40 }
  $den = if ($clay) { 0.3 } else { 0.2 }
  $inset = if ($clay) { 0.015 } else { 0.02 }
  $n = if ($clay) { 4.0 } else { 5.0 }
  $tile = [IconCut]::Detect($f.FullName, $thr, $den)
  [IconCut]::Cut($f.FullName, $tile, $inset, $n, $Size, (Join-Path $OutDir ($f.BaseName + ".png")))
}
"cut {0} icons in {1:N1} s" -f $files.Count, $sw.Elapsed.TotalSeconds
$png = Get-ChildItem $OutDir -Filter *.png
"PNG sizes: min {0:N0} KB, max {1:N0} KB, total {2:N1} MB" -f (($png | Measure-Object Length -Minimum).Minimum / 1KB), (($png | Measure-Object Length -Maximum).Maximum / 1KB), (($png | Measure-Object Length -Sum).Sum / 1MB)

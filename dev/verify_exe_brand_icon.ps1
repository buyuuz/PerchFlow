param([string]$ExePath='target\release\perch.exe')
$ErrorActionPreference='Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$ico=Join-Path $repo 'logo.ico'
$front=Join-Path $repo 'frontend\assets\logo.ico'
$exe=if([IO.Path]::IsPathRooted($ExePath)){$ExePath}else{Join-Path $repo $ExePath}
foreach($file in @($ico,$front,$exe)){if(!(Test-Path -LiteralPath $file)){throw "Missing required file: $file"}}
if((Get-FileHash -LiteralPath $ico -Algorithm SHA256).Hash -ne
   (Get-FileHash -LiteralPath $front -Algorithm SHA256).Hash){
    throw 'Windows EXE source icon differs from frontend logo.ico'
}
Add-Type -AssemblyName System.Drawing
$reference = [System.Drawing.Icon]::new($ico,[System.Drawing.Size]::new(32,32))
$embedded = [System.Drawing.Icon]::ExtractAssociatedIcon($exe)
if($null -eq $embedded){throw "No embedded icon in $exe"}
$embeddedBitmap=$embedded.ToBitmap()
$referenceBitmap=$reference.ToBitmap()
try {
    if($embeddedBitmap.Width -ne $referenceBitmap.Width -or $embeddedBitmap.Height -ne $referenceBitmap.Height){
        throw ("Unexpected icon size: EXE {0}x{1}, expected {2}x{3}" -f $embeddedBitmap.Width,$embeddedBitmap.Height,$referenceBitmap.Width,$referenceBitmap.Height)
    }
    $pixels=0
    $difference=0
    $colored=0
    for($y=0;$y -lt $referenceBitmap.Height;$y++){
        for($x=0;$x -lt $referenceBitmap.Width;$x++){
            $a=$referenceBitmap.GetPixel($x,$y)
            $b=$embeddedBitmap.GetPixel($x,$y)
            $difference += [Math]::Abs([int]$a.R-[int]$b.R)
            $difference += [Math]::Abs([int]$a.G-[int]$b.G)
            $difference += [Math]::Abs([int]$a.B-[int]$b.B)
            $difference += [Math]::Abs([int]$a.A-[int]$b.A)
            if($b.A -gt 64){$colored++}
            $pixels++
        }
    }
    $meanDifference=$difference/($pixels*4)
    if($colored -lt 80){throw "Extracted app icon unexpectedly blank: $colored colored pixels"}
    if($meanDifference -gt 18){throw ("PE icon does not match PerchFlow brand: mean channel difference {0:N1}" -f $meanDifference)}
    Write-Output ("EXE_ICON_MATCH_OK: extracted {0}x{1}; colored={2}; mean-diff={3:N2}; icon-source SHA256={4}" -f $embeddedBitmap.Width,$embeddedBitmap.Height,$colored,$meanDifference,(Get-FileHash $ico -Algorithm SHA256).Hash)
}
finally{
    $referenceBitmap.Dispose();$embeddedBitmap.Dispose();$reference.Dispose();$embedded.Dispose()
}


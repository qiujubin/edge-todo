param(
    [string]$InPath = "E:\WorkSpace\zai\edge-todo\scripts\out\screen1.png",
    [string]$OutPath = "E:\WorkSpace\zai\edge-todo\scripts\out\edge-zoom.png",
    [int]$W = 140,
    [int]$H = 560,
    [int]$Zoom = 3,
    [int]$X = -1,
    [int]$Y = -1
)
Add-Type -AssemblyName System.Drawing
$img = [System.Drawing.Image]::FromFile($InPath)
if ($X -lt 0) { $X = $img.Width - $W }
if ($Y -lt 0) { $Y = [int](($img.Height - $H) / 2) }
$ow = $W * $Zoom
$oh = $H * $Zoom
$bmp = New-Object System.Drawing.Bitmap($ow, $oh)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
$dest = New-Object System.Drawing.Rectangle(0, 0, $ow, $oh)
$region = New-Object System.Drawing.Rectangle($X, $Y, $W, $H)
$g.DrawImage($img, $dest, $region, [System.Drawing.GraphicsUnit]::Pixel)
$bmp.Save($OutPath)
$g.Dispose(); $bmp.Dispose(); $img.Dispose()
Write-Output "saved $OutPath (${ow}x${oh}) from ($X,$Y)"

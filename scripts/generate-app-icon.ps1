# Temporary product artwork; replace assets/app.svg and regenerate app.ico together.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$images = @()
foreach ($size in @(16,24,32,48,256)) {
    $bitmap = [Drawing.Bitmap]::new($size,$size)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $graphics.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $graphics.ScaleTransform($size/256.0,$size/256.0)
    $graphics.Clear([Drawing.Color]::Transparent)
    $background = [Drawing.SolidBrush]::new([Drawing.ColorTranslator]::FromHtml('#233b56'))
    $graphics.FillEllipse($background,8,8,240,240)
    $graphics.FillRectangle([Drawing.Brushes]::White,104,66,48,56)
    $graphics.FillEllipse([Drawing.Brushes]::White,104,42,48,48)
    $graphics.FillEllipse([Drawing.Brushes]::White,104,98,48,48)
    $pen = [Drawing.Pen]::new([Drawing.Color]::White,13)
    $graphics.DrawArc($pen,80,84,96,96,0,180)
    $graphics.DrawLine($pen,128,180,128,208)
    $graphics.DrawLine($pen,104,208,152,208)
    $slash = [Drawing.Pen]::new([Drawing.ColorTranslator]::FromHtml('#ff8c69'),20)
    $graphics.DrawLine($slash,65,60,195,196)
    $stream = [IO.MemoryStream]::new()
    $bitmap.Save($stream,[Drawing.Imaging.ImageFormat]::Png)
    $images += ,@($size,$stream.ToArray())
    $stream.Dispose(); $graphics.Dispose(); $bitmap.Dispose(); $pen.Dispose(); $slash.Dispose(); $background.Dispose()
}
$file = [IO.File]::Create((Join-Path (Split-Path $PSScriptRoot) 'assets/app.ico'))
$writer = [IO.BinaryWriter]::new($file)
try {
    $writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]$images.Count)
    $offset = 6 + 16*$images.Count
    foreach ($image in $images) {
        $dimension = if ($image[0] -eq 256) {0} else {$image[0]}
        $writer.Write([byte]$dimension); $writer.Write([byte]$dimension)
        $writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]32)
        $writer.Write([uint32]$image[1].Length); $writer.Write([uint32]$offset)
        $offset += $image[1].Length
    }
    foreach ($image in $images) { $writer.Write([byte[]]$image[1]) }
} finally { $writer.Dispose() }

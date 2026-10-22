# Convert supplied artwork to runtime PNGs and a multi-resolution Windows ICO.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
# A fixed, shared microphone mask preserves its white separator on every state.
# Source artwork stays untouched; only near-white background pixels are removed.
$drawingReferences = @([Drawing.Bitmap].Assembly.Location, [Drawing.Rectangle].Assembly.Location)
foreach ($name in @('System.Private.Windows.GdiPlus','System.Private.Windows.Core')) {
    try { $drawingReferences += [Reflection.Assembly]::Load($name).Location } catch { }
}
Add-Type -ReferencedAssemblies ($drawingReferences | Select-Object -Unique) -TypeDefinition @'
using System;
using System.Drawing;
using System.Drawing.Drawing2D;
public static class AlcMuteArtwork {
    public static Rectangle Bounds(Bitmap image) {
        int left=image.Width,top=image.Height,right=-1,bottom=-1;
        for(int y=0;y<image.Height;y++) for(int x=0;x<image.Width;x++) {
            if(image.GetPixel(x,y).A<=8) continue;
            left=Math.Min(left,x); top=Math.Min(top,y); right=Math.Max(right,x); bottom=Math.Max(bottom,y);
        }
        if(right<left) throw new InvalidOperationException("Empty icon");
        return Rectangle.FromLTRB(left,top,right+1,bottom+1);
    }
    public static Bitmap Prepare(string path) {
        using (var source = new Bitmap(path)) {
            var result = new Bitmap(source.Width, source.Height);
            using (var mask = new Bitmap(source.Width, source.Height)) {
                using (var g = Graphics.FromImage(mask)) {
                    g.ScaleTransform(source.Width / 1254f, source.Height / 1254f);
                    // Body and U-shaped holder, shared by all four original images.
                    using (var body = new GraphicsPath()) {
                        body.AddArc(479,377,286,286,180,180);
                        body.AddLine(765,520,765,660);
                        body.AddArc(479,517,286,286,0,180);
                        body.CloseFigure();
                        g.FillPath(Brushes.White,body);
                    }
                    using (var holder = new GraphicsPath())
                    using (var pen = new Pen(Color.White,55)) {
                        pen.StartCap = LineCap.Round; pen.EndCap = LineCap.Round;
                        holder.AddLine(465,625,465,673);
                        holder.AddBezier(465,673,465,865,789,865,789,673);
                        holder.AddLine(789,673,789,625);
                        g.DrawPath(pen,holder);
                    }
                    // Muted diagonal has a white outline as part of the symbol.
                    if (path.EndsWith("alcmute-muted.png", StringComparison.OrdinalIgnoreCase)) {
                        using (var pen = new Pen(Color.White,72)) {
                            pen.StartCap = LineCap.Round; pen.EndCap = LineCap.Round;
                            g.DrawLine(pen,472,740,765,458);
                        }
                    }
                }
                for (int y=0;y<source.Height;y++) for(int x=0;x<source.Width;x++) {
                    var c=source.GetPixel(x,y);
                    // Keep original colored pixels, internal white details and the common outline.
                    bool white=c.R>=235 && c.G>=235 && c.B>=235;
                    result.SetPixel(x,y,white && mask.GetPixel(x,y).A==0 ? Color.Transparent : c);
                }
            }
            return result;
        }
    }
}
'@
$prepared = @{}
$assetDir = Join-Path (Split-Path $PSScriptRoot) 'assets'
function Render-Png([string]$Source, [int]$Size) {
    if (-not $prepared.ContainsKey($Source)) { $prepared[$Source] = [AlcMuteArtwork]::Prepare($Source) }
    $sourceImage = $prepared[$Source]
    $bitmap = [Drawing.Bitmap]::new($Size, $Size)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $stream = [IO.MemoryStream]::new()
    try {
        $graphics.InterpolationMode = [Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $graphics.PixelOffsetMode = [Drawing.Drawing2D.PixelOffsetMode]::HighQuality
        # Trim transparent exterior space, then fit without distorting the artwork.
        $bounds = [AlcMuteArtwork]::Bounds($sourceImage)
        $left = $bounds.Left; $top = $bounds.Top; $width = $bounds.Width; $height = $bounds.Height
        $scale = $Size / [double][Math]::Max($width, $height)
        $dest = [Drawing.RectangleF]::new(($Size-$width*$scale)/2, ($Size-$height*$scale)/2, $width*$scale, $height*$scale)
        $graphics.Clear([Drawing.Color]::Transparent)
        $graphics.DrawImage($sourceImage, $dest, [Drawing.RectangleF]::new($left,$top,$width,$height), [Drawing.GraphicsUnit]::Pixel)
        $bitmap.Save($stream, [Drawing.Imaging.ImageFormat]::Png)
        return ,$stream.ToArray()
    } finally { $stream.Dispose(); $graphics.Dispose(); $bitmap.Dispose() }
}
$logo = Join-Path $assetDir 'img/alcmute-logo.png'
[IO.File]::WriteAllBytes((Join-Path $assetDir 'app.png'), (Render-Png $logo 256))
$images = @()
foreach ($size in @(16,24,32,48,256)) {
    $images += ,@($size, (Render-Png $logo $size))
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
foreach ($image in $prepared.Values) { $image.Dispose() }

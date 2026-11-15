using System;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;

// Cuts glass app-icon tiles out of flat (white/black) JPEG backgrounds.
// Detection finds the tile's square bounds; the cut uses an anti-aliased
// superellipse ("squircle") mask, inset slightly to drop background fringe.
public static class IconCut
{
    public struct Tile { public int X, Y, Side; public int BgR, BgG, BgB; public int L, T, R, B; }

    static byte[] ReadBgr(Bitmap bmp, out int stride)
    {
        var rect = new Rectangle(0, 0, bmp.Width, bmp.Height);
        var data = bmp.LockBits(rect, ImageLockMode.ReadOnly, PixelFormat.Format24bppRgb);
        stride = data.Stride;
        var buf = new byte[stride * bmp.Height];
        Marshal.Copy(data.Scan0, buf, 0, buf.Length);
        bmp.UnlockBits(data);
        return buf;
    }

    // Average of the four corner patches = background colour.
    static void Background(byte[] px, int stride, int w, int h, out int r, out int g, out int b)
    {
        long sr = 0, sg = 0, sb = 0, n = 0; int p = Math.Max(8, w / 60);
        int[][] origins = { new[] { 0, 0 }, new[] { w - p, 0 }, new[] { 0, h - p }, new[] { w - p, h - p } };
        foreach (var o in origins)
            for (int y = o[1]; y < o[1] + p; y++)
                for (int x = o[0]; x < o[0] + p; x++)
                {
                    int i = y * stride + x * 3; sb += px[i]; sg += px[i + 1]; sr += px[i + 2]; n++;
                }
        r = (int)(sr / n); g = (int)(sg / n); b = (int)(sb / n);
    }

    /// Finds the tile: rows/columns where enough pixels differ strongly from the background.
    public static Tile Detect(string path, int threshold, double density)
    {
        using (var src = new Bitmap(path))
        {
            int w = src.Width, h = src.Height, stride;
            var px = ReadBgr(src, out stride);
            int br, bg, bb; Background(px, stride, w, h, out br, out bg, out bb);
            var rows = new int[h]; var cols = new int[w];
            for (int y = 0; y < h; y++)
                for (int x = 0; x < w; x++)
                {
                    int i = y * stride + x * 3;
                    int d = Math.Max(Math.Abs(px[i + 2] - br), Math.Max(Math.Abs(px[i + 1] - bg), Math.Abs(px[i] - bb)));
                    if (d > threshold) { rows[y]++; cols[x]++; }
                }
            int minRow = (int)(w * density), minCol = (int)(h * density);
            int top = 0, bottom = h - 1, left = 0, right = w - 1;
            while (top < h - 1 && rows[top] < minRow) top++;
            while (bottom > top && rows[bottom] < minRow) bottom--;
            while (left < w - 1 && cols[left] < minCol) left++;
            while (right > left && cols[right] < minCol) right--;
            int tw = right - left + 1, th = bottom - top + 1;
            // Tiles are square. A drop shadow only adds height below the tile,
            // so the width is the side; if the span is clearly taller than wide,
            // keep the top edge, otherwise centre vertically.
            int side = tw;
            int y0 = th > tw ? top : top + (th - side) / 2;
            return new Tile { X = left, Y = y0, Side = side, BgR = br, BgG = bg, BgB = bb, L = left, T = top, R = right, B = bottom };
        }
    }

    // Superellipse |x|^n + |y|^n <= 1 on [-1,1]^2.
    static double Coverage(double u, double v, double n, double halfPixel)
    {
        double f = Math.Pow(Math.Abs(u), n) + Math.Pow(Math.Abs(v), n);
        if (f < 0.97) return 1.0;
        if (f > 1.03 + halfPixel * 8) return 0.0;
        // Edge: 4x4 supersampling.
        int inside = 0;
        for (int sy = 0; sy < 4; sy++)
            for (int sx = 0; sx < 4; sx++)
            {
                double uu = u + (sx - 1.5) * halfPixel / 2, vv = v + (sy - 1.5) * halfPixel / 2;
                if (Math.Pow(Math.Abs(uu), n) + Math.Pow(Math.Abs(vv), n) <= 1.0) inside++;
            }
        return inside / 16.0;
    }

    /// Crops the tile (inset by `inset` of its side), scales to `size`, applies the mask, saves PNG.
    public static void Cut(string path, Tile t, double inset, double exponent, int size, string outPath)
    {
        using (var src = new Bitmap(path))
        using (var dst = new Bitmap(size, size, PixelFormat.Format32bppArgb))
        {
            double inPx = t.Side * inset;
            var srcRect = new RectangleF((float)(t.X + inPx), (float)(t.Y + inPx), (float)(t.Side - 2 * inPx), (float)(t.Side - 2 * inPx));
            using (var g = Graphics.FromImage(dst))
            {
                g.CompositingMode = CompositingMode.SourceCopy;
                g.InterpolationMode = InterpolationMode.HighQualityBicubic;
                g.PixelOffsetMode = PixelOffsetMode.HighQuality;
                using (var attrs = new ImageAttributes())
                {
                    attrs.SetWrapMode(WrapMode.TileFlipXY); // no dark/bright seams at the crop edge
                    g.DrawImage(src, new Rectangle(0, 0, size, size), srcRect.X, srcRect.Y, srcRect.Width, srcRect.Height, GraphicsUnit.Pixel, attrs);
                }
            }
            var rect = new Rectangle(0, 0, size, size);
            var data = dst.LockBits(rect, ImageLockMode.ReadWrite, PixelFormat.Format32bppArgb);
            var buf = new byte[data.Stride * size];
            Marshal.Copy(data.Scan0, buf, 0, buf.Length);
            double half = 1.0 / size; // half a pixel in [-1,1] units
            for (int y = 0; y < size; y++)
            {
                double v = (y + 0.5) / size * 2 - 1;
                for (int x = 0; x < size; x++)
                {
                    double u = (x + 0.5) / size * 2 - 1;
                    double a = Coverage(u, v, exponent, half);
                    int i = y * data.Stride + x * 4;
                    buf[i + 3] = (byte)Math.Round(buf[i + 3] * a);
                }
            }
            Marshal.Copy(buf, 0, data.Scan0, buf.Length);
            dst.UnlockBits(data);
            dst.Save(outPath, ImageFormat.Png);
        }
    }
}

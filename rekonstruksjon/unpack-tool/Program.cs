using Iced.Intel;
using System.Text;

// usage: dis <file> <offsetHex> <lenHex> <bits> [baseHex]
//        hex <file> <offsetHex> <lenHex>
//        strings <file> [min]
var mode = args[0];
var data = File.ReadAllBytes(args[1]);
if (mode == "dis")
{
    int off = Convert.ToInt32(args[2], 16);
    int len = Convert.ToInt32(args[3], 16);
    int bits = int.Parse(args[4]);
    ulong bas = args.Length > 5 ? Convert.ToUInt64(args[5], 16) : (ulong)off;
    len = Math.Min(len, data.Length - off);
    var reader = new ByteArrayCodeReader(data, off, len);
    var dec = Iced.Intel.Decoder.Create(bits, reader);
    dec.IP = bas;
    var fmt = new MasmFormatter();
    fmt.Options.FirstOperandCharIndex = 8;
    var output = new StringOutput();
    ulong end = bas + (ulong)len;
    while (dec.IP < end)
    {
        dec.Decode(out var ins);
        fmt.Format(ins, output);
        int start = off + (int)(ins.IP - bas);
        var hex = BitConverter.ToString(data, start, ins.Length).Replace("-", "");
        Console.WriteLine($"{ins.IP:X8} {hex,-20} {output.ToStringAndReset()}");
    }
}
else if (mode == "hex")
{
    int off = Convert.ToInt32(args[2], 16);
    int len = Convert.ToInt32(args[3], 16);
    for (int o = off; o < Math.Min(off + len, data.Length); o += 16)
    {
        var sb = new StringBuilder();
        var asc = new StringBuilder();
        for (int i = 0; i < 16 && o + i < data.Length; i++)
        {
            byte b = data[o + i];
            sb.Append($"{b:X2} ");
            asc.Append(b >= 32 && b < 127 ? (char)b : '.');
        }
        Console.WriteLine($"{o:X6}: {sb,-48} {asc}");
    }
}
else if (mode == "unpack")
{
    // unpack <file> <outImage> <rolCount>
    int mzSize = (data[4] | data[5] << 8) - 1;
    int hdr = mzSize * 512 + (data[2] | data[3] << 8);
    int pos = hdr + BitConverter.ToInt32(data, hdr + 0x10);
    int imageSize = BitConverter.ToInt32(data, hdr + 0x20);
    int memSize = BitConverter.ToInt32(data, hdr + 0x18);
    Console.WriteLine($"hdr={hdr:X} data={pos:X} eip={BitConverter.ToInt32(data, hdr + 0x14):X} esp={BitConverter.ToInt32(data, hdr + 0x1C):X} img={imageSize:X} mem={memSize:X}");
    var o = new List<byte>();
    int bp = 0, cnt = 1;
    int GetBit()
    {
        int b = bp & 1; bp >>= 1;
        if (--cnt == 0) { bp = data[pos] | data[pos + 1] << 8; pos += 2; cnt = 16; }
        return b;
    }
    void Copy(int off, int n) { for (int i = 0; i < n; i++) o.Add(o[o.Count + off]); }
    GetBit();
    while (true)
    {
        if (GetBit() == 1) { o.Add(data[pos++]); continue; }
        int cf = GetBit();
        byte al = data[pos++];
        byte bh = 0xFF, bl = al;
        int len;
        if (cf == 1)
        {
            bh = (byte)((bh << 1) | GetBit());
            if (GetBit() == 0)
            {
                byte dh = 2;
                for (int c = 0; c < 3; c++)
                {
                    if (GetBit() == 1) break;
                    bh = (byte)((bh << 1) | GetBit());
                    dh <<= 1;
                }
                bh -= dh;
            }
            int d = 2; len = -1;
            for (int c = 0; c < 4; c++) { d++; if (GetBit() == 1) { len = d; break; } }
            if (len < 0)
            {
                if (GetBit() == 1) { d++; if (GetBit() == 1) d++; len = d; }
                else if (GetBit() == 1) len = data[pos++] + 0x11;
                else { int v = 0; for (int c = 0; c < 3; c++) v = v << 1 | GetBit(); len = v + 9; }
            }
        }
        else
        {
            if (GetBit() == 1)
            {
                for (int c = 0; c < 3; c++) bh = (byte)((bh << 1) | GetBit());
                bh--; len = 2;
            }
            else if (bh != bl) len = 2;
            else
            {
                if (GetBit() == 0) break;
                continue;
            }
        }
        int off = unchecked((int)(0xFFFF0000u | (uint)(bh << 8) | bl));
        Copy(off, len);
    }
    Console.WriteLine($"unpacked {o.Count:X} bytes, consumed to {pos:X} of {data.Length:X}");
    var img = o.ToArray();
    int rol = int.Parse(args[3]);
    var relocs = new List<int>();
    int esi = imageSize, edi = 0;
    while (esi < img.Length)
    {
        uint eax = 0;
        while (true)
        {
            byte b = img[esi++];
            b = (byte)((b << rol) | (b >> (8 - rol)));
            eax = (eax & 0xFFFFFF00) | b;
            if ((eax & 1) != 0) { eax <<= 7; continue; }
            break;
        }
        eax >>= 1; edi += (int)eax;
        if (esi != img.Length && img[esi] == 0) { esi++; relocs.Add(-edi); }
        else relocs.Add(edi);
    }
    Console.WriteLine($"relocs={relocs.Count} last={edi:X} esi={esi:X}");
    File.WriteAllBytes(args[2], img.Take(imageSize).ToArray());
    File.WriteAllLines(args[2] + ".relocs", relocs.Select(r => r < 0 ? $"SEL {-r:X}" : $"{r:X}"));
}
else if (mode == "strings")
{
    int min = args.Length > 2 ? int.Parse(args[2]) : 4;
    var cur = new StringBuilder(); int st = 0;
    for (int i = 0; i <= data.Length; i++)
    {
        byte c = i < data.Length ? data[i] : (byte)0;
        if (c >= 32 && c < 127) { if (cur.Length == 0) st = i; cur.Append((char)c); }
        else { if (cur.Length >= min) Console.WriteLine($"{st:X6} {cur}"); cur.Clear(); }
    }
}

# Enkel lokal webserver for testing: kopierer invit.wasm og musikken inn i web/
# og serverer mappa på http://localhost:8070/
param([int]$Port = 8070)

$web = $PSScriptRoot
$root = Split-Path $web
Copy-Item "$root\invit-rs\target\wasm32-unknown-unknown\release\invit.wasm" $web
Copy-Item "$root\DATA\HIGH_HPS.XM" $web

$types = @{ '.html' = 'text/html; charset=utf-8'; '.js' = 'text/javascript'; '.wasm' = 'application/wasm' }
$listener = [System.Net.HttpListener]::new()
$listener.Prefixes.Add("http://localhost:$Port/")
$listener.Start()
Write-Host "Serverer $web på http://localhost:$Port/"
while ($listener.IsListening) {
    $ctx = $listener.GetContext()
    $name = [Uri]::UnescapeDataString($ctx.Request.Url.AbsolutePath.TrimStart('/'))
    if ($name -eq '') { $name = 'index.html' }
    $file = Join-Path $web $name
    if ((Test-Path $file -PathType Leaf) -and ((Resolve-Path $file).Path.StartsWith($web))) {
        $bytes = [IO.File]::ReadAllBytes($file)
        $ext = [IO.Path]::GetExtension($file).ToLower()
        $ctx.Response.ContentType = if ($types[$ext]) { $types[$ext] } else { 'application/octet-stream' }
        $ctx.Response.Headers.Add('Cache-Control', 'no-store')
        $ctx.Response.OutputStream.Write($bytes, 0, $bytes.Length)
    } else {
        $ctx.Response.StatusCode = 404
    }
    $ctx.Response.Close()
}

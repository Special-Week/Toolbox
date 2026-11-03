$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot '..')
try {
    $architecture = (& gcc -dumpmachine).Trim()
    if ($LASTEXITCODE -ne 0) { throw '无法读取 GCC 架构。' }
    $objectDirectory = Join-Path 'build' ('pdf-' + $architecture)
    New-Item -ItemType Directory -Path $objectDirectory -Force | Out-Null
    $sources = @(Get-ChildItem third_party/pdfio/pdfio-*.c, third_party/pdfio/ttf/*.c, third_party/zlib/*.c)
    $objects = @()
    foreach ($source in $sources) {
        $object = Join-Path $objectDirectory ($source.BaseName + '.o')
        # 依赖源码随项目固定；头文件变更也重建，避免复用另一版本的对象文件。
        $headers = Get-ChildItem third_party/pdfio/*.h, third_party/pdfio/ttf/*.h, third_party/zlib/*.h
        $newest = @($source) + @($headers) | Sort-Object LastWriteTime -Descending | Select-Object -First 1
        if (-not (Test-Path -LiteralPath $object) -or (Get-Item -LiteralPath $object).LastWriteTime -lt $newest.LastWriteTime) {
            & gcc -std=c11 -O2 -w -D_WIN32_WINNT=0x0601 -DWINVER=0x0601 -Ithird_party/pdfio -Ithird_party/pdfio/ttf -Ithird_party/zlib -c $source.FullName -o $object
            if ($LASTEXITCODE -ne 0) { throw "PDF 依赖编译失败：$($source.Name)" }
        }
        $objects += $object
    }
    & ar rcs build/pdf-deps.a @objects
    if ($LASTEXITCODE -ne 0) { throw 'PDF 依赖归档失败。' }
} finally {
    Pop-Location
}

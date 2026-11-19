# Install or update clock-in:
#   irm https://raw.githubusercontent.com/churnware/clock-in/main/install.ps1 | iex
#
# Downloads the latest release to %LOCALAPPDATA%\clock-in\bin, checks its
# SHA-256, and adds that folder to your user PATH. Run it again to update.

& {
    $ErrorActionPreference = 'Stop'
    $ProgressPreference = 'SilentlyContinue'
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

    $repo = 'churnware/clock-in'
    $bin = Join-Path $env:LOCALAPPDATA 'clock-in\bin'
    $exe = Join-Path $bin 'clock-in.exe'

    Write-Host ''
    Write-Host '  clock-in' -ForegroundColor Magenta -NoNewline
    Write-Host ' installer'
    Write-Host ''

    $release = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest" `
        -Headers @{ 'User-Agent' = 'clock-in-installer' }
    $asset = $release.assets | Where-Object name -eq 'clock-in.exe'
    $sums = $release.assets | Where-Object name -eq 'clock-in.exe.sha256'
    if (-not $asset -or -not $sums) {
        throw "release $($release.tag_name) is missing clock-in.exe or its checksum"
    }

    $tmp = Join-Path ([IO.Path]::GetTempPath()) "clock-in-$([guid]::NewGuid()).exe"
    try {
        Write-Host "  downloading $($release.tag_name)..."
        Invoke-WebRequest $asset.browser_download_url -OutFile $tmp -UseBasicParsing
        $expected = ((Invoke-RestMethod $sums.browser_download_url) -split '\s+')[0].Trim().ToLower()
        $actual = (Get-FileHash $tmp -Algorithm SHA256).Hash.ToLower()
        if ($expected -ne $actual) {
            throw "checksum mismatch (expected $expected, got $actual)"
        }

        # A running scheduler holds the exe open; stop it and start it again after.
        $wasRunning = $false
        if (Test-Path $exe) {
            $wasRunning = (& $exe status) -match 'scheduler running'
            if ($wasRunning) { & $exe stop | Out-Null; Start-Sleep -Milliseconds 500 }
        }

        New-Item -ItemType Directory -Force $bin | Out-Null
        Move-Item $tmp $exe -Force
    } finally {
        if (Test-Path $tmp) { Remove-Item $tmp -Force }
    }

    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $entries = @($userPath -split ';' | Where-Object { $_ })
    if ($entries -notcontains $bin) {
        [Environment]::SetEnvironmentVariable('Path', (($entries + $bin) -join ';'), 'User')
        Write-Host "  added $bin to your PATH"
    }
    if (($env:Path -split ';') -notcontains $bin) { $env:Path += ";$bin" }

    if ($wasRunning) { & $exe start | Out-Null }

    Write-Host ''
    Write-Host "  installed clock-in $($release.tag_name)" -ForegroundColor Green
    Write-Host '  run ' -NoNewline
    Write-Host 'clock-in' -ForegroundColor Cyan -NoNewline
    Write-Host ' to set your hours'
    Write-Host ''
}

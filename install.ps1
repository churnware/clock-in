# Install or update clock-in:
#   irm https://raw.githubusercontent.com/churnware/clock-in/main/install.ps1 | iex
#
# Downloads the latest release to %LOCALAPPDATA%\clock-in\bin, checks its
# SHA-256, and adds that folder to your user PATH. Run it again to update.
# Kept to ASCII so Windows PowerShell 5.1 reads it correctly when piped.

& {
    $ErrorActionPreference = 'Stop'
    $ProgressPreference = 'SilentlyContinue'
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

    $repo = 'churnware/clock-in'
    $bin = Join-Path $env:LOCALAPPDATA 'clock-in\bin'
    $exe = Join-Path $bin 'clock-in.exe'

    # -- Look ---------------------------------------------------------------

    $fancy = -not $env:NO_COLOR -and -not [Console]::IsOutputRedirected
    $e = [char]27
    function rgb([int]$r, [int]$g, [int]$b) { if ($fancy) { "$e[38;2;$r;$g;${b}m" } else { '' } }
    $sky = rgb 0x8A 0xB4 0xF8
    $light = rgb 0x7B 0x83 0xEB
    $indigo = rgb 0x5B 0x5F 0xC7
    $brand = rgb 0x4B 0x53 0xBC
    $lavender = rgb 0xA3 0x8C 0xF8
    $muted = rgb 0xA4 0xA6 0xD0
    $grey = rgb 0xF5 0xF5 0xF5
    $rose = rgb 0xF2 0x8F 0xB1
    $bold = if ($fancy) { "$e[1m" } else { '' }
    $reset = if ($fancy) { "$e[0m" } else { '' }
    $dot = [char]0x2022
    $tick = if ($fancy) { "$([char]0x2713) " } else { '' }

    # Lucky cat, two paw positions. Columns 0-9 are the body, 10+ the paw.
    $pawUp = @(
        '   /\_/\    __ ',
        '  ( EwE )  (  )',
        '  /  o  \  / / ',
        ' (_______)/_/  '
    )
    $pawDown = @(
        '   /\_/\       ',
        '  ( EwE )   __ ',
        '  /  o  \  (__)',
        ' (_______)/_/  '
    )
    $shades = @($sky, $light, $indigo, $brand)

    function Cat-Row([string[]]$sprite, [int]$row, [bool]$blink) {
        $eye = if ($blink) { '-' } else { $dot }
        $body = $sprite[$row].Substring(0, 10).Replace('E', "$bold$grey$eye$reset$($shades[$row])")
        if ($row -eq 2) { $body = $body.Replace('o', "$bold$lavender" + 'o' + "$reset$($shades[$row])") }
        "$($shades[$row])$body$sky$($sprite[$row].Substring(10))$reset"
    }

    $script:lines = @("$bold${grey}clock-in$reset$muted installer$reset", '', '', '')
    $script:drawn = $false

    function Draw([int]$frame) {
        $sprite = if ([math]::Floor($frame / 2) % 2 -eq 0) { $pawUp } else { $pawDown }
        $blink = ($frame % 23) -eq 22
        if ($script:drawn -and $fancy) { [Console]::Write("$e[4A") }
        for ($i = 0; $i -lt 4; $i++) {
            $clear = if ($fancy) { "$e[2K" } else { '' }
            [Console]::WriteLine("$clear  $(Cat-Row $sprite $i $blink)   $($script:lines[$i])")
        }
        $script:drawn = $true
    }

    function Say([int]$row, [string]$text) {
        $script:lines[$row] = $text
        if ($fancy) { Draw 0 } else { Write-Host ($text -replace "$e\[[0-9;]*m", '') }
    }

    # -- Install ------------------------------------------------------------

    $tmp = Join-Path ([IO.Path]::GetTempPath()) "clock-in-$([guid]::NewGuid()).exe"
    $frame = 0
    $encoding = [Console]::OutputEncoding
    if ($fancy) { [Console]::OutputEncoding = [Text.Encoding]::UTF8; [Console]::Write("$e[?25l") }
    [Console]::WriteLine()
    if (-not $fancy) { Write-Host 'clock-in installer' }
    try {
        Say 1 "${muted}finding the latest release$reset"
        $release = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest" `
            -Headers @{ 'User-Agent' = 'clock-in-installer' }
        $tag = $release.tag_name
        $asset = $release.assets | Where-Object name -eq 'clock-in.exe'
        $sums = $release.assets | Where-Object name -eq 'clock-in.exe.sha256'
        if (-not $asset -or -not $sums) { throw "release $tag is missing clock-in.exe or its checksum" }

        # Download in the background so the cat can wave meanwhile.
        Say 1 "${light}downloading $bold$tag$reset"
        $web = New-Object Net.WebClient
        $web.Headers.Add('User-Agent', 'clock-in-installer')
        $task = $web.DownloadFileTaskAsync($asset.browser_download_url, $tmp)
        while (-not $task.IsCompleted) {
            if ($fancy) { Draw $frame; $frame++ }
            Start-Sleep -Milliseconds 143
        }
        $task.GetAwaiter().GetResult()

        Say 1 "${light}checking sha-256$reset"
        $expected = ((Invoke-RestMethod $sums.browser_download_url) -split '\s+')[0].Trim().ToLower()
        $actual = (Get-FileHash $tmp -Algorithm SHA256).Hash.ToLower()
        if ($expected -ne $actual) { throw "checksum mismatch (expected $expected, got $actual)" }

        # A running scheduler holds the exe open; stop it and start it again after.
        $wasRunning = $false
        if (Test-Path $exe) {
            $wasRunning = (& $exe status) -match 'scheduler running'
            if ($wasRunning) { & $exe stop | Out-Null; Start-Sleep -Milliseconds 500 }
        }
        New-Item -ItemType Directory -Force $bin | Out-Null
        Move-Item $tmp $exe -Force

        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        $entries = @($userPath -split ';' | Where-Object { $_ })
        $pathNote = "${muted}in $bin$reset"
        if ($entries -notcontains $bin) {
            [Environment]::SetEnvironmentVariable('Path', (($entries + $bin) -join ';'), 'User')
            $pathNote = "${muted}added $bin to PATH$reset"
        }
        if (($env:Path -split ';') -notcontains $bin) { $env:Path += ";$bin" }
        if ($wasRunning) { & $exe start | Out-Null }

        $script:lines[1] = "$bold$lavender${tick}installed $tag$reset"
        $script:lines[2] = $pathNote
        $script:lines[3] = "${muted}run $reset$bold${sky}clock-in$reset$muted to set your hours$reset"
        if ($fancy) {
            for ($i = 0; $i -lt 14; $i++) { Draw $frame; $frame++; Start-Sleep -Milliseconds 143 }
            Draw 0
        } else {
            $script:lines[1..3] | ForEach-Object { Write-Host ($_ -replace "$e\[[0-9;]*m", '') }
        }
    } catch {
        $script:lines[1] = "$bold${rose}install failed$reset"
        $script:lines[2] = "$rose$($_.Exception.Message)$reset"
        $script:lines[3] = ''
        if ($fancy) { Draw 0 } else { Write-Host "install failed: $($_.Exception.Message)" }
    } finally {
        if (Test-Path $tmp) { Remove-Item $tmp -Force }
        if ($fancy) { [Console]::Write("$e[?25h") }
        [Console]::WriteLine()
        [Console]::OutputEncoding = $encoding
    }
}

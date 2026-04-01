#Requires -Version 5.1
<#
.SYNOPSIS
    TimeForged Installer for Windows
.DESCRIPTION
    Fully autonomous: installs all dependencies (Rust, Node.js, jq),
    builds, configures, starts daemon + tray, sets up autostart.
.USAGE
    git clone <repo>; cd timeforged; .\install.ps1
#>

$ErrorActionPreference = "Stop"

function Write-Header($msg) { Write-Host "`n  $msg" -ForegroundColor DarkYellow }
function Write-Ok($msg) { Write-Host "  ✓ " -ForegroundColor Green -NoNewline; Write-Host $msg }
function Write-Info($msg) { Write-Host "  ▸ " -ForegroundColor DarkYellow -NoNewline; Write-Host $msg }
function Write-Warn($msg) { Write-Host "  ! " -ForegroundColor Yellow -NoNewline; Write-Host $msg }
function Write-Fail($msg) { Write-Host "  ✗ " -ForegroundColor Red -NoNewline; Write-Host $msg; exit 1 }

# Detect available package manager
function Get-PkgManager {
    if (Get-Command winget -ErrorAction SilentlyContinue) { return "winget" }
    if (Get-Command scoop -ErrorAction SilentlyContinue) { return "scoop" }
    if (Get-Command choco -ErrorAction SilentlyContinue) { return "choco" }
    return $null
}

function Install-Pkg($name, $wingetId, $scoopName, $chocoName) {
    $mgr = Get-PkgManager
    switch ($mgr) {
        "winget" {
            Write-Info "Installing $name via winget..."
            winget install --id $wingetId --accept-source-agreements --accept-package-agreements -e 2>&1 | Out-Null
        }
        "scoop" {
            Write-Info "Installing $name via scoop..."
            scoop install $scoopName 2>&1 | Out-Null
        }
        "choco" {
            Write-Info "Installing $name via chocolatey..."
            choco install $chocoName -y 2>&1 | Out-Null
        }
        default {
            return $false
        }
    }
    # Refresh PATH
    $env:PATH = [Environment]::GetEnvironmentVariable("PATH", "Machine") + ";" + [Environment]::GetEnvironmentVariable("PATH", "User")
    return $true
}

Write-Host ""
Write-Host "  ╔══════════════════════════════════════╗" -ForegroundColor DarkYellow
Write-Host "  ║         TimeForged Installer          ║" -ForegroundColor DarkYellow
Write-Host "  ║     self-hosted time tracker           ║" -ForegroundColor DarkYellow
Write-Host "  ╚══════════════════════════════════════╝" -ForegroundColor DarkYellow
Write-Host ""

$InstallDir = "$env:LOCALAPPDATA\TimeForged\bin"
$ConfigDir = "$env:APPDATA\timeforged"
$StartupDir = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup"

# ══════════════════════════════════════
# 1. INSTALL ALL DEPENDENCIES
# ══════════════════════════════════════
Write-Header "Installing dependencies..."

$mgr = Get-PkgManager
if ($mgr) {
    Write-Ok "Package manager: $mgr"
} else {
    Write-Warn "No package manager (winget/scoop/choco) found"
    Write-Info "Will try direct installers for missing tools"
}

# ── Git ──
if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    if (-not (Install-Pkg "Git" "Git.Git" "git" "git")) {
        Write-Fail "Git not found. Install from https://git-scm.com/download/win"
    }
}
Write-Ok "Git $(git --version)"

# ── Rust / Cargo ──
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Info "Rust not found — installing via rustup..."

    # Try package manager first
    $installed = Install-Pkg "Rust" "Rustlang.Rustup" "rustup" "rustup.install"

    if (-not $installed -or -not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        # Direct download rustup-init.exe
        Write-Info "Downloading rustup-init.exe..."
        $rustupUrl = "https://win.rustup.rs/x86_64"
        $rustupExe = "$env:TEMP\rustup-init.exe"
        Invoke-WebRequest -Uri $rustupUrl -OutFile $rustupExe -UseBasicParsing
        & $rustupExe -y --default-toolchain stable 2>&1 | Select-Object -Last 3
        Remove-Item $rustupExe -Force -ErrorAction SilentlyContinue

        # Add cargo to PATH for this session
        $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
    }

    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Fail "Cargo still not found. Restart your terminal and re-run."
    }
    Write-Ok "Rust installed $((rustc --version) -replace 'rustc\s+','' -replace '\s.*','')"
} else {
    Write-Ok "Rust $((rustc --version) -replace 'rustc\s+','' -replace '\s.*','')"
}

# ── Node.js / npm ──
if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    Write-Info "Node.js not found — installing..."

    $installed = Install-Pkg "Node.js" "OpenJS.NodeJS.LTS" "nodejs-lts" "nodejs-lts"

    if (-not $installed -or -not (Get-Command node -ErrorAction SilentlyContinue)) {
        # Direct download
        Write-Info "Downloading Node.js LTS installer..."
        $nodeUrl = "https://nodejs.org/dist/v22.15.0/node-v22.15.0-x64.msi"
        $nodeMsi = "$env:TEMP\node-install.msi"
        Invoke-WebRequest -Uri $nodeUrl -OutFile $nodeMsi -UseBasicParsing
        Start-Process msiexec.exe -ArgumentList "/i `"$nodeMsi`" /qn" -Wait
        Remove-Item $nodeMsi -Force -ErrorAction SilentlyContinue

        # Refresh PATH
        $env:PATH = [Environment]::GetEnvironmentVariable("PATH", "Machine") + ";" + [Environment]::GetEnvironmentVariable("PATH", "User")
    }

    if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
        Write-Fail "Node.js still not found. Restart your terminal and re-run."
    }
    Write-Ok "Node.js installed $(node --version)"
} else {
    Write-Ok "Node $(node --version) / npm $(npm --version)"
}

# ── jq ──
$HasJq = $true
if (-not (Get-Command jq -ErrorAction SilentlyContinue)) {
    $installed = Install-Pkg "jq" "jqlang.jq" "jq" "jq"

    if (-not $installed -or -not (Get-Command jq -ErrorAction SilentlyContinue)) {
        # Direct download jq binary
        Write-Info "Downloading jq binary..."
        $jqDir = "$env:LOCALAPPDATA\jq"
        New-Item -ItemType Directory -Path $jqDir -Force | Out-Null
        $jqUrl = "https://github.com/jqlang/jq/releases/latest/download/jq-windows-amd64.exe"
        Invoke-WebRequest -Uri $jqUrl -OutFile "$jqDir\jq.exe" -UseBasicParsing
        $env:PATH = "$jqDir;$env:PATH"
        $userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
        if ($userPath -notlike "*jq*") {
            [Environment]::SetEnvironmentVariable("PATH", "$userPath;$jqDir", "User")
        }
    }

    if (Get-Command jq -ErrorAction SilentlyContinue) {
        Write-Ok "jq installed $(jq --version)"
    } else {
        Write-Warn "Could not install jq — Claude Code hooks will be skipped"
        $HasJq = $false
    }
} else {
    Write-Ok "jq $(jq --version)"
}

# ══════════════════════════════════════
# 2. PROJECT ROOT
# ══════════════════════════════════════
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if (Test-Path "$ScriptDir\Cargo.toml") {
    $ProjectDir = $ScriptDir
} else {
    $ProjectDir = Get-Location
}
if (-not (Test-Path "$ProjectDir\Cargo.toml")) {
    Write-Fail "Not in TimeForged project root."
}
Set-Location $ProjectDir
Write-Ok "Project root: $ProjectDir"

# ══════════════════════════════════════
# 3. BUILD WEB DASHBOARD
# ══════════════════════════════════════
Write-Header "Building web dashboard..."

$WebDir = "$ProjectDir\crates\timeforged\web"
if (-not (Test-Path $WebDir)) { Write-Fail "Web directory not found" }

Push-Location $WebDir
Write-Info "Installing npm dependencies..."
npm install --silent 2>&1 | Out-Null
Write-Ok "Dependencies installed"

Write-Info "Building Vue app..."
npx vite build --logLevel error 2>&1 | Out-Null
Write-Ok "Dashboard built"
Pop-Location

# ══════════════════════════════════════
# 4. BUILD RUST BINARIES
# ══════════════════════════════════════
Write-Header "Building Rust binaries..."

Write-Info "Compiling (this may take a few minutes on first run)..."
$prev = $ErrorActionPreference; $ErrorActionPreference = "Continue"
cargo build --release 2>&1 | Select-String "Compiling|Finished" | ForEach-Object { Write-Host "    $_" -ForegroundColor DarkGray }
$ErrorActionPreference = $prev
if ($LASTEXITCODE -ne 0) { Write-Fail "cargo build failed (exit code $LASTEXITCODE)" }
Write-Ok "Binaries built"

$DaemonBin = "$ProjectDir\target\release\timeforged.exe"
$CliBin = "$ProjectDir\target\release\tf.exe"
$TrayBin = "$ProjectDir\target\release\timeforged-tray.exe"

if (-not (Test-Path $DaemonBin)) { Write-Fail "Daemon binary not found" }
if (-not (Test-Path $CliBin)) { Write-Fail "CLI binary not found" }
if (-not (Test-Path $TrayBin)) { Write-Fail "Tray binary not found" }
Write-Ok "All binaries built successfully"

# ══════════════════════════════════════
# 5. INSTALL BINARIES
# ══════════════════════════════════════
Write-Header "Installing binaries..."

New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
Copy-Item $DaemonBin "$InstallDir\timeforged.exe" -Force
Copy-Item $CliBin "$InstallDir\tf.exe" -Force
Copy-Item $TrayBin "$InstallDir\timeforged-tray.exe" -Force
Write-Ok "Installed to $InstallDir"

$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notlike "*TimeForged*") {
    [Environment]::SetEnvironmentVariable("PATH", "$UserPath;$InstallDir", "User")
    $env:PATH = "$env:PATH;$InstallDir"
    Write-Ok "Added to user PATH"
}

# ══════════════════════════════════════
# 6. FIRST RUN — START DAEMON
# ══════════════════════════════════════
Write-Header "Starting TimeForged daemon..."

Get-Process timeforged -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1

$DaemonLog = [System.IO.Path]::GetTempFileName()
$DaemonProc = Start-Process -FilePath "$InstallDir\timeforged.exe" `
    -RedirectStandardOutput $DaemonLog `
    -RedirectStandardError ([System.IO.Path]::GetTempFileName()) `
    -WindowStyle Hidden -PassThru
Start-Sleep -Seconds 3

if ($DaemonProc.HasExited) {
    Get-Content $DaemonLog
    Write-Fail "Daemon failed to start"
}
Write-Ok "Daemon running (PID: $($DaemonProc.Id))"

$ApiKey = ""
$LogContent = Get-Content $DaemonLog -Raw -ErrorAction SilentlyContinue
if ($LogContent -match "tf_\w+") { $ApiKey = $Matches[0] }

$CliConfig = "$ConfigDir\cli.toml"
if ([string]::IsNullOrEmpty($ApiKey) -and (Test-Path $CliConfig)) {
    $existing = Select-String -Path $CliConfig -Pattern 'api_key\s*=\s*"([^"]+)"' -ErrorAction SilentlyContinue
    if ($existing) { $ApiKey = $existing.Matches[0].Groups[1].Value }
}

if (-not [string]::IsNullOrEmpty($ApiKey)) {
    New-Item -ItemType Directory -Path $ConfigDir -Force | Out-Null
    @"
server_url = "http://127.0.0.1:6175"
api_key = "$ApiKey"
"@ | Set-Content $CliConfig -Encoding UTF8
    Write-Ok "CLI config saved"
}

try {
    Invoke-RestMethod -Uri "http://127.0.0.1:6175/health" -TimeoutSec 3 | Out-Null
    Write-Ok "Health check passed"
} catch {
    Write-Fail "Daemon not responding on port 6175"
}
Remove-Item $DaemonLog -Force -ErrorAction SilentlyContinue

# ══════════════════════════════════════
# 7. AUTOSTART
# ══════════════════════════════════════
Write-Header "Setting up autostart..."

# Daemon — Task Scheduler
Write-Info "Creating daemon scheduled task..."
try {
    $taskAction = New-ScheduledTaskAction -Execute "$InstallDir\timeforged.exe"
    $taskTrigger = New-ScheduledTaskTrigger -AtLogOn
    $taskSettings = New-ScheduledTaskSettingsSet -RestartCount 3 -RestartInterval (New-TimeSpan -Minutes 1)
    Register-ScheduledTask -TaskName "TimeForged Daemon" -Action $taskAction -Trigger $taskTrigger `
        -Settings $taskSettings -Force | Out-Null
    Write-Ok "Daemon → Task Scheduler (at logon)"
} catch {
    Write-Warn "Could not create scheduled task — start timeforged.exe manually"
}

# Tray — Startup folder
Write-Info "Adding tray to Startup..."
$ws = New-Object -ComObject WScript.Shell
$shortcut = $ws.CreateShortcut("$StartupDir\TimeForged Tray.lnk")
$shortcut.TargetPath = "$InstallDir\timeforged-tray.exe"
$shortcut.Description = "TimeForged system tray"
$shortcut.Save()
Write-Ok "Tray → Startup folder"

Stop-Process -Name "timeforged-tray" -Force -ErrorAction SilentlyContinue
Start-Process -FilePath "$InstallDir\timeforged-tray.exe" -WindowStyle Hidden
Write-Ok "Tray app started"

# ══════════════════════════════════════
# 8. REMOTE SYNC
# ══════════════════════════════════════
Write-Header "Setting up remote sync..."

$DefaultRemote = "https://timeforged.blysspeak.space"
$RemoteUrl = Read-Host "  ▸ Remote server [$DefaultRemote]"
if ([string]::IsNullOrWhiteSpace($RemoteUrl)) { $RemoteUrl = $DefaultRemote }

$RemoteOk = $false
$RemoteKey = ""
$TfUsername = ""

try {
    Invoke-RestMethod -Uri "$RemoteUrl/health" -TimeoutSec 5 | Out-Null
    Write-Ok "Remote server reachable"

    Write-Host ""
    Write-Host "  [1] New account  — create a new username on the server"
    Write-Host "  [2] Link         — connect to an existing account with an API key"
    Write-Host "  [3] Skip         — set up remote sync later"
    Write-Host ""
    $Choice = Read-Host "  ▸ Choose [1/2/3]"

    switch ($Choice) {
        "1" {
            $TfUsername = Read-Host "  ▸ Choose a username"
            if (-not [string]::IsNullOrWhiteSpace($TfUsername)) {
                Write-Info "Registering as $TfUsername..."
                try {
                    $regOutput = & "$InstallDir\tf.exe" register $TfUsername --remote $RemoteUrl 2>&1 | Out-String
                    if ($regOutput -match "tf_\w+") {
                        $RemoteKey = $Matches[0]
                        Write-Ok "Registered as $TfUsername"
                        $RemoteOk = $true
                    } else {
                        Write-Warn "Registration succeeded but could not extract API key"
                    }
                } catch {
                    Write-Warn "Registration failed: $_"
                }
            }
        }
        "2" {
            $RemoteKey = Read-Host "  ▸ Paste your remote API key (tf_...)"
            if ($RemoteKey -match "^tf_\w+$") {
                Write-Info "Linking to existing account..."
                try {
                    $linkOutput = & "$InstallDir\tf.exe" link $RemoteKey --remote $RemoteUrl 2>&1 | Out-String
                    Write-Ok "Linked to existing account"
                    $RemoteOk = $true
                } catch {
                    Write-Warn "Link failed: $_"
                }
            } else {
                Write-Warn "Invalid key format (expected tf_...)"
            }
        }
        default {
            Write-Info "Skipping remote setup"
            Write-Info "Run later: tf register <username> --remote $RemoteUrl"
            Write-Info "Or link:   tf link <api-key> --remote $RemoteUrl"
        }
    }
} catch {
    Write-Warn "Remote unreachable — skipping."
    Write-Info "Run later: tf register <username> --remote $RemoteUrl"
    Write-Info "Or link:   tf link <api-key> --remote $RemoteUrl"
}

if ($RemoteOk -and -not [string]::IsNullOrEmpty($ApiKey)) {
    if ([string]::IsNullOrEmpty($RemoteKey)) {
        # Link command saves config itself, just need remote_url
        $existing = Get-Content $CliConfig -Raw -ErrorAction SilentlyContinue
        if ($existing -notmatch "remote_url") {
            Add-Content $CliConfig "`nremote_url = `"$RemoteUrl`""
        }
    } else {
        @"
server_url = "http://127.0.0.1:6175"
api_key = "$ApiKey"
remote_url = "$RemoteUrl"
remote_key = "$RemoteKey"
"@ | Set-Content $CliConfig -Encoding UTF8
    }
    Write-Ok "Config updated with remote"

    & "$InstallDir\tf.exe" profile --public 2>&1 | Out-Null
    & "$InstallDir\tf.exe" sync 2>&1 | Out-Null

    # Auto-sync task
    try {
        $syncAction = New-ScheduledTaskAction -Execute "$InstallDir\tf.exe" -Argument "sync"
        $syncTrigger = New-ScheduledTaskTrigger -Once -At (Get-Date) -RepetitionInterval (New-TimeSpan -Minutes 15)
        Register-ScheduledTask -TaskName "TimeForged Sync" -Action $syncAction -Trigger $syncTrigger -Force | Out-Null
        Write-Ok "Auto-sync task (every 15 min)"
    } catch {
        Write-Warn "Could not create sync task"
    }
}

# ══════════════════════════════════════
# 9. CLAUDE CODE HOOKS
# ══════════════════════════════════════
$ClaudeDir = "$env:USERPROFILE\.claude"
if ((Test-Path $ClaudeDir) -and $HasJq) {
    Write-Header "Configuring Claude Code hooks..."

    New-Item -ItemType Directory -Path "$ClaudeDir\hooks" -Force | Out-Null
    Copy-Item "$ProjectDir\contrib\claude-code\timeforged-heartbeat.sh" "$ClaudeDir\hooks\timeforged-heartbeat.sh" -Force
    Write-Ok "Hook script installed"

    $ClaudeSettings = "$ClaudeDir\settings.json"
    $HookCmd = ("$ClaudeDir\hooks\timeforged-heartbeat.sh") -replace '\\', '/'
    $HookConfig = jq -n --arg cmd $HookCmd '{hooks:{UserPromptSubmit:[{matcher:"",hooks:[{type:"command",command:$cmd,timeout:5}]}],PostToolUse:[{matcher:"",hooks:[{type:"command",command:$cmd,timeout:5}]}],Stop:[{matcher:"",hooks:[{type:"command",command:$cmd,timeout:5}]}]}}'

    if (Test-Path $ClaudeSettings) {
        $tmp = [System.IO.Path]::GetTempFileName()
        $hooks = $HookConfig | jq '.hooks'
        jq --argjson hooks $hooks '.hooks = (.hooks // {}) * $hooks' $ClaudeSettings | Set-Content $tmp
        Move-Item $tmp $ClaudeSettings -Force
    } else {
        $HookConfig | Set-Content $ClaudeSettings -Encoding UTF8
    }
    Write-Ok "Claude Code hooks configured"
}

# ══════════════════════════════════════
# SUMMARY
# ══════════════════════════════════════
Write-Host ""
Write-Host "  ╔══════════════════════════════════════╗" -ForegroundColor DarkYellow
Write-Host "  ║       Installation Complete!          ║" -ForegroundColor DarkYellow
Write-Host "  ╚══════════════════════════════════════╝" -ForegroundColor DarkYellow
Write-Host ""

Write-Host "  Dashboard:   " -NoNewline; Write-Host "http://127.0.0.1:6175" -ForegroundColor Green
if (-not [string]::IsNullOrEmpty($ApiKey)) {
    Write-Host "  API Key:     " -NoNewline; Write-Host $ApiKey -ForegroundColor DarkYellow
}
if ($RemoteOk) {
    Write-Host "  Card URL:    " -NoNewline
    Write-Host "$($RemoteUrl.TrimEnd('/'))/github/timeforged/$TfUsername.svg" -ForegroundColor Green
    Write-Host "  Auto-sync:   every 15 min"
}
Write-Host ""
Write-Host "  Config:      $ConfigDir\cli.toml" -ForegroundColor DarkGray
Write-Host "  Binaries:    $InstallDir" -ForegroundColor DarkGray
Write-Host "  Tray:        " -NoNewline; Write-Host "running in system tray" -ForegroundColor Green
Write-Host ""

if ($RemoteOk) {
    Write-Host "  GitHub README:" -ForegroundColor White
    Write-Host "    <img src=`"$($RemoteUrl.TrimEnd('/'))/github/timeforged/$TfUsername.svg`" />" -ForegroundColor Green
    Write-Host ""
}

Write-Host "  Quick start:" -ForegroundColor White
Write-Host "    tf init C:\projects" -ForegroundColor Green -NoNewline; Write-Host "  — start tracking"
Write-Host "    tf today" -ForegroundColor DarkGray -NoNewline; Write-Host "            — today's summary"
Write-Host "    tf status" -ForegroundColor DarkGray -NoNewline; Write-Host "           — daemon status"
Write-Host ""
Write-Host "  Autostart:" -ForegroundColor White
Write-Host "    Daemon → Task Scheduler (at logon)" -ForegroundColor DarkGray
Write-Host "    Tray   → Startup folder" -ForegroundColor DarkGray
Write-Host "    Right-click tray → Open Dashboard / Quit" -ForegroundColor DarkGray
Write-Host ""

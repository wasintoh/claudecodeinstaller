<#
.SYNOPSIS
    Claude Code Installer for Windows - One-command setup for non-developers.

.DESCRIPTION
    Installs Git for Windows, Node.js LTS, and Claude Code with a single command.
    Handles PATH configuration, version checks, and provides clear error messages.

    Designed for Thai learners in a Claude Code training program who may have
    never used a terminal before.

.PARAMETER Uninstall
    Enter uninstall mode to remove installed components.

.PARAMETER SkipNode
    Skip Node.js installation.

.PARAMETER DryRun
    Simulate installation without making changes.

.EXAMPLE
    # Quick install (piped from web):
    irm https://raw.githubusercontent.com/user/repo/main/claude-installer-ps/install-claude-code.ps1 | iex

    # Install from saved file:
    .\install-claude-code.ps1

    # Install without Node.js:
    .\install-claude-code.ps1 -SkipNode

    # Dry run (see what would happen):
    .\install-claude-code.ps1 -DryRun

    # Uninstall:
    .\install-claude-code.ps1 -Uninstall

.NOTES
    Version: 1.0.0
    Requires: Windows 10 1809+ or Windows 11, PowerShell 5.1+
    No administrator privileges required for the main install flow.
#>

# Script-level param block: works when run as a saved file.
# When piped via `irm | iex`, this is silently ignored (no args passed).
param(
    [switch]$Uninstall,
    [switch]$SkipNode,
    [switch]$DryRun
)

function Invoke-ClaudeCodeInstaller {
    [CmdletBinding()]
    param(
        [switch]$Uninstall,
        [switch]$SkipNode,
        [switch]$DryRun
    )

    # ═══════════════════════════════════════════════════════════════════════════
    # GLOBAL CONFIGURATION
    # ═══════════════════════════════════════════════════════════════════════════

    $Script:InstallerVersion = "1.0.0"
    $Script:LogFile = Join-Path $env:TEMP "claude-code-install.log"
    $Script:TempDir = Join-Path $env:TEMP "claude-installer"
    $Script:ManifestPath = Join-Path $env:USERPROFILE ".claude-installer\manifest.json"
    $Script:DryRunMode = $DryRun.IsPresent
    $Script:NonInteractive = [string]::IsNullOrEmpty($PSCommandPath)

    # Critical for PS 5.1: progress bar can slow downloads by 10x
    $ProgressPreference = 'SilentlyContinue'

    # Ensure temp directory exists
    if (-not (Test-Path $Script:TempDir)) {
        New-Item -ItemType Directory -Path $Script:TempDir -Force | Out-Null
    }

    # Initialize log file
    $null = New-Item -ItemType File -Path $Script:LogFile -Force

    # Manifest tracking for uninstall
    $Script:Manifest = @{
        installedAt   = (Get-Date).ToString('o')
        scriptVersion = $Script:InstallerVersion
        components    = @{
            git       = @{ installed = $false; version = ''; preExisting = $false }
            node      = @{ installed = $false; version = ''; preExisting = $false }
            npm       = @{ installed = $false; version = '' }
            claudeCode = @{ installed = $false; version = ''; preExisting = $false }
        }
        pathEntries   = @()
    }

    # ═══════════════════════════════════════════════════════════════════════════
    # LOGGING & DISPLAY FUNCTIONS
    # ═══════════════════════════════════════════════════════════════════════════

    function Write-Log {
        param([string]$Message)
        $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
        $entry = "[$timestamp] $Message"
        Add-Content -Path $Script:LogFile -Value $entry -ErrorAction SilentlyContinue
    }

    function Write-Step {
        param(
            [int]$Number,
            [int]$Total,
            [string]$Message
        )
        $prefix = "[$Number/$Total]"
        Write-Host ""
        Write-Host "  $prefix " -ForegroundColor Cyan -NoNewline
        Write-Host $Message -ForegroundColor White
        Write-Host "  $('-' * 54)" -ForegroundColor DarkGray
        Write-Log "STEP $prefix $Message"
    }

    function Write-Status {
        param(
            [string]$Message,
            [switch]$Success,
            [switch]$Warning,
            [switch]$Error,
            [switch]$Info
        )
        $icon = "  "
        $color = "White"
        $level = "INFO"

        if ($Success) { $icon = "  [OK]"; $color = "Green"; $level = "OK" }
        elseif ($Warning) { $icon = "  [!!]"; $color = "Yellow"; $level = "WARN" }
        elseif ($Error) { $icon = "  [XX]"; $color = "Red"; $level = "ERROR" }
        elseif ($Info) { $icon = "  [--]"; $color = "Cyan"; $level = "INFO" }

        Write-Host "$icon " -ForegroundColor $color -NoNewline
        Write-Host $Message
        Write-Log "$level $Message"
    }

    function Write-ErrorMessage {
        param([string]$Message, [string]$Suggestion)
        Write-Host ""
        Write-Host "  [XX] ERROR: $Message" -ForegroundColor Red
        if ($Suggestion) {
            Write-Host "       $Suggestion" -ForegroundColor Yellow
        }
        Write-Host ""
        Write-Log "ERROR: $Message | Suggestion: $Suggestion"
    }

    function Show-Banner {
        $banner = @"

    ================================================================

       _____ _                 _         ____          _
      / ____| |               | |       / ___|___   __| | ___
     | |    | | __ _ _   _  __| | ___  | |   / _ \ / _` |/ _ \
     | |    | |/ _` | | | |/ _` |/ _ \ | |__| (_) | (_| |  __/
      \____|_|\__,_|\__,_|\__,_|\___/  \____\___/ \__,_|\___|

              I N S T A L L E R   f o r   W i n d o w s
                          Version $($Script:InstallerVersion)

    ================================================================

"@
        Write-Host $banner -ForegroundColor Cyan
        if ($Script:DryRunMode) {
            Write-Host "    >>> DRY RUN MODE - No changes will be made <<<" -ForegroundColor Yellow
            Write-Host ""
        }
        Write-Log "=== Claude Code Installer v$($Script:InstallerVersion) started ==="
        Write-Log "Mode: $(if ($Script:NonInteractive) { 'Non-Interactive (piped)' } else { 'Interactive (saved file)' })"
        Write-Log "DryRun: $($Script:DryRunMode)"
    }

    # ═══════════════════════════════════════════════════════════════════════════
    # UTILITY FUNCTIONS
    # ═══════════════════════════════════════════════════════════════════════════

    function Invoke-DownloadWithRetry {
        param(
            [string]$Url,
            [string]$OutFile,
            [string]$Description = "file",
            [int]$MaxRetries = 3
        )

        Write-Status "Downloading $Description..." -Info
        Write-Log "Download: $Url -> $OutFile"

        $delays = @(1, 3, 9)  # Exponential backoff per spec

        for ($attempt = 1; $attempt -le $MaxRetries; $attempt++) {
            try {
                # Clean up partial downloads
                if (Test-Path $OutFile) {
                    Remove-Item -Path $OutFile -Force -ErrorAction SilentlyContinue
                }

                if ($Script:DryRunMode) {
                    Write-Status "[DRY RUN] Would download $Description from $Url" -Info
                    return
                }

                Invoke-WebRequest -Uri $Url -OutFile $OutFile -UseBasicParsing -ErrorAction Stop

                # Verify download
                if (-not (Test-Path $OutFile)) {
                    throw "Downloaded file not found at $OutFile"
                }

                $fileSize = (Get-Item $OutFile).Length
                if ($fileSize -eq 0) {
                    throw "Downloaded file is empty (0 bytes)"
                }

                # Unblock to prevent SmartScreen issues
                Unblock-File -Path $OutFile -ErrorAction SilentlyContinue

                $sizeMB = [math]::Round($fileSize / 1MB, 1)
                Write-Status "Downloaded $Description ($sizeMB MB)" -Success
                Write-Log "Download complete: $sizeMB MB"
                return
            }
            catch {
                $errorMsg = $_.Exception.Message
                Write-Log "Download attempt $attempt/$MaxRetries failed: $errorMsg"

                if (Test-Path $OutFile) {
                    Remove-Item -Path $OutFile -Force -ErrorAction SilentlyContinue
                }

                if ($attempt -lt $MaxRetries) {
                    $delay = $delays[$attempt - 1]
                    Write-Status "Download attempt $attempt failed. Retrying in ${delay}s..." -Warning
                    Start-Sleep -Seconds $delay
                }
            }
        }

        throw "Failed to download $Description after $MaxRetries attempts. URL: $Url"
    }

    function Compare-SemVer {
        param(
            [string]$Version1,
            [string]$Version2
        )
        # Strip leading 'v' and anything after dash (pre-release)
        $v1str = ($Version1 -replace '^v', '') -replace '-.*$', ''
        $v2str = ($Version2 -replace '^v', '') -replace '-.*$', ''

        try {
            $v1 = [version]$v1str
            $v2 = [version]$v2str
            return $v1.CompareTo($v2)
        }
        catch {
            Write-Log "Version comparison failed: '$Version1' vs '$Version2': $_"
            return 0
        }
    }

    function Get-SystemArch {
        if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') {
            return 'arm64'
        }
        if ([Environment]::Is64BitOperatingSystem) {
            return 'x64'
        }
        throw "Claude Code requires a 64-bit version of Windows. Your system appears to be 32-bit."
    }

    function Get-CommandPath {
        param([string]$CommandName)
        try {
            $cmd = Get-Command $CommandName -ErrorAction Stop
            return $cmd.Source
        }
        catch {
            return $null
        }
    }

    function Get-CommandVersion {
        param([string]$CommandName, [string]$VersionArg = '--version')
        try {
            $output = & $CommandName $VersionArg 2>&1
            $versionMatch = [regex]::Match("$output", '(\d+\.\d+[\.\d]*)')
            if ($versionMatch.Success) {
                return $versionMatch.Groups[1].Value
            }
            return $null
        }
        catch {
            return $null
        }
    }

    function Read-UserChoice {
        param(
            [string]$Prompt,
            [string]$Default = 'Y'
        )
        if ($Script:NonInteractive) {
            Write-Log "Non-interactive mode: auto-selecting '$Default' for: $Prompt"
            return $Default
        }
        $response = Read-Host "  $Prompt"
        if ([string]::IsNullOrWhiteSpace($response)) {
            return $Default
        }
        return $response.Trim().ToUpper()
    }

    # ═══════════════════════════════════════════════════════════════════════════
    # PATH MANAGEMENT FUNCTIONS
    # ═══════════════════════════════════════════════════════════════════════════

    function Add-ToUserPath {
        param([string]$Directory)

        $normalized = $Directory.TrimEnd('\')
        Write-Log "Add-ToUserPath: checking '$normalized'"

        # Read current User PATH from registry
        $currentPath = [System.Environment]::GetEnvironmentVariable('Path', 'User')
        if (-not $currentPath) { $currentPath = '' }

        $entries = $currentPath -split ';' | Where-Object { $_ -ne '' }

        # Case-insensitive check for existing entry
        $alreadyExists = $entries | Where-Object { $_.TrimEnd('\') -ieq $normalized }
        if ($alreadyExists) {
            Write-Log "Add-ToUserPath: '$normalized' already in PATH"
            return $false
        }

        if ($Script:DryRunMode) {
            Write-Status "[DRY RUN] Would add to PATH: $normalized" -Info
            return $true
        }

        $newPath = (($entries + $normalized) | Where-Object { $_ -ne '' }) -join ';'

        try {
            [System.Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
            Write-Log "Add-ToUserPath: added '$normalized'"

            # Track for manifest
            $Script:Manifest.pathEntries += $normalized
            return $true
        }
        catch {
            Write-Log "Add-ToUserPath FAILED: $_"
            return $false
        }
    }

    function Remove-FromUserPath {
        param([string]$Directory)

        $normalized = $Directory.TrimEnd('\')
        Write-Log "Remove-FromUserPath: removing '$normalized'"

        $currentPath = [System.Environment]::GetEnvironmentVariable('Path', 'User')
        if (-not $currentPath) { return $false }

        $entries = $currentPath -split ';' | Where-Object { $_ -ne '' }
        $filtered = $entries | Where-Object { $_.TrimEnd('\') -ine $normalized }

        if ($filtered.Count -eq $entries.Count) {
            Write-Log "Remove-FromUserPath: '$normalized' not found in PATH"
            return $false
        }

        if ($Script:DryRunMode) {
            Write-Status "[DRY RUN] Would remove from PATH: $normalized" -Info
            return $true
        }

        $newPath = ($filtered | Where-Object { $_ -ne '' }) -join ';'

        try {
            [System.Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
            Write-Log "Remove-FromUserPath: removed '$normalized'"
            return $true
        }
        catch {
            Write-Log "Remove-FromUserPath FAILED: $_"
            return $false
        }
    }

    function Refresh-PathFromRegistry {
        $machinePath = [System.Environment]::GetEnvironmentVariable('Path', 'Machine')
        $userPath = [System.Environment]::GetEnvironmentVariable('Path', 'User')
        $env:PATH = "$machinePath;$userPath"
        Write-Log "PATH refreshed from registry"
    }

    function Send-SettingChange {
        Write-Log "Broadcasting WM_SETTINGCHANGE..."
        try {
            if (-not ('Win32.NativeMethods' -as [type])) {
                Add-Type -Namespace Win32 -Name NativeMethods -MemberDefinition @'
[DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Auto)]
public static extern IntPtr SendMessageTimeout(
    IntPtr hWnd, uint Msg, UIntPtr wParam, string lParam,
    uint fuFlags, uint uTimeout, out UIntPtr lpdwResult);
'@
            }
            $HWND_BROADCAST = [IntPtr]0xffff
            $WM_SETTINGCHANGE = 0x001A
            $SMTO_ABORTIFHUNG = 0x0002
            $result = [UIntPtr]::Zero
            [Win32.NativeMethods]::SendMessageTimeout(
                $HWND_BROADCAST, $WM_SETTINGCHANGE, [UIntPtr]::Zero,
                'Environment', $SMTO_ABORTIFHUNG, 5000, [ref]$result
            ) | Out-Null
            Write-Log "WM_SETTINGCHANGE broadcast sent"
        }
        catch {
            Write-Log "WM_SETTINGCHANGE failed (non-critical): $_"
        }
    }

    # ═══════════════════════════════════════════════════════════════════════════
    # PHASE 0: PRE-FLIGHT CHECKS
    # ═══════════════════════════════════════════════════════════════════════════

    function Invoke-PreflightChecks {
        Write-Step -Number 1 -Total 6 -Message "Running pre-flight checks..."

        $allPassed = $true

        # Check 1: PowerShell (not CMD)
        $shellName = $host.Name
        if ($shellName -notmatch 'ConsoleHost|Visual Studio Code|Windows Terminal') {
            Write-Status "Unexpected shell detected: $shellName" -Warning
            Write-Log "Shell: $shellName"
        }
        else {
            Write-Status "PowerShell detected ($shellName)" -Success
        }

        # Check 2: Windows version (build >= 17763 = Windows 10 1809)
        try {
            $osBuild = [Environment]::OSVersion.Version.Build
            $osVersion = [Environment]::OSVersion.Version
            if ($osBuild -ge 17763) {
                Write-Status "Windows version: $osVersion (build $osBuild)" -Success
            }
            else {
                Write-Status "Windows version $osVersion is too old. Requires Windows 10 1809+ (build 17763+)" -Error
                $allPassed = $false
            }
        }
        catch {
            Write-Status "Could not determine Windows version: $_" -Warning
        }

        # Check 3: Execution Policy
        try {
            $policy = Get-ExecutionPolicy -Scope CurrentUser
            $machinePolicy = Get-ExecutionPolicy -Scope LocalMachine
            if ($policy -eq 'Restricted' -and $machinePolicy -eq 'Restricted') {
                Write-Status "Execution Policy is Restricted" -Warning
                Write-Host "       To fix, run this command as Administrator:" -ForegroundColor Yellow
                Write-Host "       Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser" -ForegroundColor White
                Write-Host "       Or run the script with: powershell -ExecutionPolicy Bypass -File .\install-claude-code.ps1" -ForegroundColor White
                # Not a hard failure if we're already running (irm | iex bypasses this)
            }
            else {
                Write-Status "Execution Policy: $policy (CurrentUser), $machinePolicy (LocalMachine)" -Success
            }
        }
        catch {
            Write-Status "Could not check Execution Policy: $_" -Warning
        }

        # Check 4: Internet connectivity (test actual endpoints we'll use)
        try {
            $endpoints = @('https://api.github.com', 'https://nodejs.org', 'https://claude.ai')
            $connected = $false
            foreach ($ep in $endpoints) {
                try {
                    $null = Invoke-WebRequest -Uri $ep -UseBasicParsing -Method Head -TimeoutSec 10 -ErrorAction Stop
                    $connected = $true
                    break
                }
                catch { continue }
            }
            if ($connected) {
                Write-Status "Internet connectivity: OK" -Success
            }
            else {
                throw "All endpoint checks failed"
            }
        }
        catch {
            Write-Status "No internet connection detected" -Error
            Write-Host "       Please check your network connection and try again." -ForegroundColor Yellow
            Write-Host "       If you're behind a proxy, configure it in Windows Settings > Network & Internet > Proxy" -ForegroundColor Yellow
            $allPassed = $false
        }

        # Check 5: RAM (>= 4 GB)
        try {
            $ramGB = [math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1GB, 1)
            if ($ramGB -ge 4) {
                Write-Status "RAM: ${ramGB} GB" -Success
            }
            else {
                Write-Status "RAM: ${ramGB} GB (minimum 4 GB recommended)" -Warning
            }
        }
        catch {
            Write-Status "Could not check RAM: $_" -Warning
        }

        # Check 6: Disk space (>= 2 GB free on C:)
        try {
            $freeGB = [math]::Round((Get-PSDrive C).Free / 1GB, 1)
            if ($freeGB -ge 2) {
                Write-Status "Disk space (C:): ${freeGB} GB free" -Success
            }
            else {
                Write-Status "Low disk space: ${freeGB} GB free (need at least 2 GB)" -Error
                $allPassed = $false
            }
        }
        catch {
            Write-Status "Could not check disk space: $_" -Warning
        }

        # Check 7: Architecture
        try {
            $arch = Get-SystemArch
            Write-Status "Architecture: $arch" -Success
        }
        catch {
            Write-Status "$($_.Exception.Message)" -Error
            $allPassed = $false
        }

        Write-Host ""
        if (-not $allPassed) {
            Write-ErrorMessage "Some pre-flight checks failed." "Please fix the issues above and try again."
            Write-Log "Pre-flight checks FAILED"
            return $false
        }

        Write-Status "All pre-flight checks passed!" -Success
        Write-Log "Pre-flight checks passed"
        return $true
    }

    # ═══════════════════════════════════════════════════════════════════════════
    # PHASE 1: GIT FOR WINDOWS
    # ═══════════════════════════════════════════════════════════════════════════

    function Install-GitComponent {
        Write-Step -Number 2 -Total 6 -Message "Checking Git for Windows..."

        # Check if git is already installed
        $gitVersion = Get-CommandVersion -CommandName 'git'
        if ($gitVersion) {
            Write-Status "Git for Windows is already installed: v$gitVersion" -Success
            $Script:Manifest.components.git.installed = $true
            $Script:Manifest.components.git.version = $gitVersion
            $Script:Manifest.components.git.preExisting = $true
            return $true
        }

        Write-Status "Git for Windows is not installed" -Warning
        Write-Status "Preparing to install Git for Windows..." -Info

        if ($Script:DryRunMode) {
            Write-Status "[DRY RUN] Would install Git for Windows (latest stable)" -Info
            return $true
        }

        try {
            # Discover latest release from GitHub API
            Write-Status "Checking latest Git version..." -Info
            try {
                $releaseInfo = Invoke-RestMethod -Uri 'https://api.github.com/repos/git-for-windows/git/releases/latest' -UseBasicParsing -ErrorAction Stop
            }
            catch {
                $statusCode = $_.Exception.Response.StatusCode.value__
                if ($statusCode -eq 403) {
                    Write-Status "GitHub API rate limit exceeded. Using fallback download page." -Warning
                    throw "GitHub API rate limited. Please download Git manually from: https://git-scm.com/download/win"
                }
                throw
            }

            $arch = Get-SystemArch
            $assetPattern = if ($arch -eq 'arm64') { 'Git-.*-arm64\.exe$' } else { 'Git-.*-64-bit\.exe$' }
            $asset = $releaseInfo.assets | Where-Object { $_.name -match $assetPattern } | Select-Object -First 1

            # Fallback to 64-bit if arm64 asset not found
            if (-not $asset -and $arch -eq 'arm64') {
                Write-Status "ARM64 installer not found, using 64-bit version" -Warning
                $asset = $releaseInfo.assets | Where-Object { $_.name -match 'Git-.*-64-bit\.exe$' } | Select-Object -First 1
            }

            if (-not $asset) {
                throw "Could not find Git installer in the latest release"
            }

            $gitTagVersion = $releaseInfo.tag_name -replace '^v', '' -replace '\.windows\.\d+$', ''
            Write-Status "Latest Git version: $gitTagVersion" -Info

            # Download
            $installerPath = Join-Path $Script:TempDir "git-installer.exe"
            Invoke-DownloadWithRetry -Url $asset.browser_download_url -OutFile $installerPath -Description "Git for Windows v$gitTagVersion"

            # Silent install (per-user, no admin required)
            Write-Status "Installing Git for Windows (this may take a minute)..." -Info
            $gitInstallDir = Join-Path $env:LOCALAPPDATA "Programs\Git"
            $gitLogFile = Join-Path $Script:TempDir "git-install.log"

            $gitArgs = @(
                '/VERYSILENT',
                '/NORESTART',
                '/SP-',
                '/SUPPRESSMSGBOXES',
                '/CLOSEAPPLICATIONS',
                '/CURRENTUSER',
                "/DIR=`"$gitInstallDir`"",
                '/COMPONENTS=gitlfs',
                "/LOG=`"$gitLogFile`""
            )

            $process = Start-Process -FilePath $installerPath -ArgumentList $gitArgs -Wait -PassThru -NoNewWindow
            Write-Log "Git installer exit code: $($process.ExitCode)"

            if ($process.ExitCode -ne 0) {
                throw "Git installer exited with code $($process.ExitCode). Check log: $gitLogFile"
            }

            # Refresh PATH and verify
            Refresh-PathFromRegistry

            $gitVersion = Get-CommandVersion -CommandName 'git'
            if (-not $gitVersion) {
                # Manual PATH fix
                Write-Status "Git installed but not in PATH. Fixing..." -Warning
                $gitCmdPath = Join-Path $gitInstallDir "cmd"
                if (Test-Path $gitCmdPath) {
                    Add-ToUserPath $gitCmdPath | Out-Null
                }
                # Also check Program Files
                $gitCmdPathGlobal = Join-Path $env:ProgramFiles "Git\cmd"
                if (Test-Path $gitCmdPathGlobal) {
                    Add-ToUserPath $gitCmdPathGlobal | Out-Null
                }

                Refresh-PathFromRegistry
                $gitVersion = Get-CommandVersion -CommandName 'git'
            }

            if ($gitVersion) {
                Write-Status "Git for Windows installed successfully: v$gitVersion" -Success
                $Script:Manifest.components.git.installed = $true
                $Script:Manifest.components.git.version = $gitVersion
                $Script:Manifest.components.git.preExisting = $false
                return $true
            }
            else {
                throw "Git installed but verification failed. You may need to restart your terminal."
            }
        }
        catch {
            Write-ErrorMessage "Failed to install Git for Windows: $($_.Exception.Message)" `
                "Please download and install manually from: https://git-scm.com/download/win"
            Write-Log "Git installation FAILED: $_"
            return $false
        }
    }

    # ═══════════════════════════════════════════════════════════════════════════
    # PHASE 2: NODE.JS LTS
    # ═══════════════════════════════════════════════════════════════════════════

    function Configure-NpmGlobalPrefix {
        # Ensures npm global prefix is set to ~/.npm-global and added to PATH.
        # Runs for both fresh and pre-existing Node.js installs (pain point #6).
        try {
            $npmCmd = Get-CommandPath -CommandName 'npm'
            if (-not $npmCmd) { return }

            $npmGlobalDir = Join-Path $env:USERPROFILE ".npm-global"
            if (-not (Test-Path $npmGlobalDir)) {
                if ($Script:DryRunMode) {
                    Write-Status "[DRY RUN] Would create npm global directory: $npmGlobalDir" -Info
                    return
                }
                New-Item -ItemType Directory -Path $npmGlobalDir -Force | Out-Null
            }

            if ($Script:DryRunMode) {
                Write-Status "[DRY RUN] Would configure npm global prefix: $npmGlobalDir" -Info
                return
            }

            & npm config set prefix "$npmGlobalDir" 2>&1 | Out-Null
            $added = Add-ToUserPath $npmGlobalDir
            if ($added) { Refresh-PathFromRegistry }
            Write-Status "npm global directory configured: $npmGlobalDir" -Success
            Write-Log "npm prefix set to $npmGlobalDir"
        }
        catch {
            Write-Status "Could not configure npm global prefix: $_" -Warning
            Write-Log "npm prefix config failed: $_"
        }
    }

    function Install-NodeComponent {
        param([switch]$Skip)

        Write-Step -Number 3 -Total 6 -Message "Checking Node.js..."

        if ($Skip) {
            Write-Status "Skipping Node.js installation (-SkipNode)" -Info
            Write-Log "Node.js skipped by user"
            return $true
        }

        # Check if Node.js is already installed
        $nodeVersion = Get-CommandVersion -CommandName 'node'
        if ($nodeVersion) {
            $comparison = Compare-SemVer $nodeVersion "18.0.0"
            if ($comparison -ge 0) {
                Write-Status "Node.js is already installed: v$nodeVersion" -Success
                $Script:Manifest.components.node.installed = $true
                $Script:Manifest.components.node.version = $nodeVersion
                $Script:Manifest.components.node.preExisting = $true

                # Also check npm
                $npmVersion = Get-CommandVersion -CommandName 'npm'
                if ($npmVersion) {
                    Write-Status "npm: v$npmVersion" -Success
                    $Script:Manifest.components.npm.installed = $true
                    $Script:Manifest.components.npm.version = $npmVersion
                }

                # Ensure npm global prefix is configured even for pre-existing installs
                Configure-NpmGlobalPrefix

                return $true
            }
            else {
                Write-Status "Node.js v$nodeVersion is outdated (minimum: v18.0.0)" -Warning
                $answer = Read-UserChoice -Prompt "Upgrade Node.js to the latest LTS version? [Y/n]" -Default 'Y'
                if ($answer -ne 'Y') {
                    Write-Status "Skipping Node.js upgrade" -Info
                    return $true
                }
            }
        }
        else {
            Write-Status "Node.js is not installed" -Warning
        }

        Write-Status "Preparing to install Node.js LTS..." -Info

        if ($Script:DryRunMode) {
            Write-Status "[DRY RUN] Would install Node.js LTS (latest)" -Info
            return $true
        }

        try {
            # Discover latest LTS version
            Write-Status "Checking latest Node.js LTS version..." -Info
            $nodeIndex = Invoke-RestMethod -Uri 'https://nodejs.org/dist/index.json' -UseBasicParsing -ErrorAction Stop

            $ltsEntry = $nodeIndex | Where-Object { $_.lts -ne $false } | Select-Object -First 1
            if (-not $ltsEntry) {
                throw "Could not determine latest Node.js LTS version"
            }

            $nodeVer = $ltsEntry.version -replace '^v', ''
            $ltsName = $ltsEntry.lts
            Write-Status "Latest Node.js LTS: v$nodeVer ($ltsName)" -Info

            # Build download URL
            $arch = Get-SystemArch
            $msiArch = if ($arch -eq 'arm64') { 'arm64' } else { 'x64' }
            $msiFileName = "node-v$nodeVer-$msiArch.msi"
            $downloadUrl = "https://nodejs.org/dist/v$nodeVer/$msiFileName"

            # Download
            $msiPath = Join-Path $Script:TempDir $msiFileName
            Invoke-DownloadWithRetry -Url $downloadUrl -OutFile $msiPath -Description "Node.js v$nodeVer LTS ($ltsName)"

            # Silent install — may require admin for system-wide MSI
            Write-Status "Installing Node.js (this may take a minute)..." -Info
            Write-Status "Note: Node.js may request administrator permission via a UAC popup." -Info
            $msiArgs = "/i `"$msiPath`" /quiet /norestart"
            $process = Start-Process -FilePath "msiexec.exe" -ArgumentList $msiArgs -Wait -PassThru -NoNewWindow
            Write-Log "Node.js MSI exit code: $($process.ExitCode)"

            # Exit code 3010 = success but needs reboot (acceptable)
            # Exit code 1603 = fatal error (often permission-related)
            # Exit code 1612 = install source unavailable
            if ($process.ExitCode -eq 1603) {
                Write-Status "Node.js installation failed — likely needs administrator privileges." -Error
                Write-Host "       Try one of these options:" -ForegroundColor Yellow
                Write-Host "       1. Right-click PowerShell > 'Run as Administrator' and run this script again" -ForegroundColor White
                Write-Host "       2. Double-click the downloaded MSI to install manually: $msiPath" -ForegroundColor White
                throw "Node.js MSI failed with exit code 1603 (permission denied)"
            }
            elseif ($process.ExitCode -ne 0 -and $process.ExitCode -ne 3010) {
                throw "Node.js installer exited with code $($process.ExitCode)"
            }

            if ($process.ExitCode -eq 3010) {
                Write-Status "Node.js installed (a reboot may be recommended later)" -Warning
            }

            # Refresh PATH and verify
            Refresh-PathFromRegistry

            $nodeVersion = Get-CommandVersion -CommandName 'node'
            $npmVersion = Get-CommandVersion -CommandName 'npm'

            if (-not $nodeVersion) {
                # Try adding common Node.js paths manually
                Write-Status "Node.js installed but not in PATH. Fixing..." -Warning
                $nodePaths = @(
                    (Join-Path $env:ProgramFiles "nodejs"),
                    (Join-Path ${env:ProgramFiles(x86)} "nodejs"),
                    (Join-Path $env:APPDATA "nodejs")
                )
                foreach ($np in $nodePaths) {
                    if (Test-Path $np) {
                        Add-ToUserPath $np | Out-Null
                    }
                }
                Refresh-PathFromRegistry
                $nodeVersion = Get-CommandVersion -CommandName 'node'
                $npmVersion = Get-CommandVersion -CommandName 'npm'
            }

            if ($nodeVersion) {
                Write-Status "Node.js installed successfully: v$nodeVersion" -Success
                $Script:Manifest.components.node.installed = $true
                $Script:Manifest.components.node.version = $nodeVersion
                $Script:Manifest.components.node.preExisting = $false

                if ($npmVersion) {
                    Write-Status "npm: v$npmVersion" -Success
                    $Script:Manifest.components.npm.installed = $true
                    $Script:Manifest.components.npm.version = $npmVersion
                }

                # Configure npm global prefix
                Configure-NpmGlobalPrefix

                return $true
            }
            else {
                throw "Node.js installed but verification failed. You may need to restart your terminal."
            }
        }
        catch {
            Write-ErrorMessage "Failed to install Node.js: $($_.Exception.Message)" `
                "Please download and install manually from: https://nodejs.org/en/download/"
            Write-Log "Node.js installation FAILED: $_"
            return $false
        }
    }

    # ═══════════════════════════════════════════════════════════════════════════
    # PHASE 3: CLAUDE CODE
    # ═══════════════════════════════════════════════════════════════════════════

    function Install-ClaudeCodeComponent {
        Write-Step -Number 4 -Total 6 -Message "Checking Claude Code..."

        # Check if Claude Code is already installed
        $claudeVersion = Get-CommandVersion -CommandName 'claude'
        if ($claudeVersion) {
            Write-Status "Claude Code is already installed: v$claudeVersion" -Success
            $Script:Manifest.components.claudeCode.installed = $true
            $Script:Manifest.components.claudeCode.version = $claudeVersion
            $Script:Manifest.components.claudeCode.preExisting = $true
            return $true
        }

        Write-Status "Claude Code is not installed" -Warning
        Write-Status "Installing Claude Code via official installer..." -Info

        if ($Script:DryRunMode) {
            Write-Status "[DRY RUN] Would install Claude Code via https://claude.ai/install.ps1" -Info
            return $true
        }

        try {
            # Download and execute the official bootstrap script in a child process
            # to avoid scope pollution and nested stdin conflicts
            Write-Status "Downloading Claude Code installer..." -Info
            $bootstrapScript = Invoke-RestMethod -Uri 'https://claude.ai/install.ps1' -UseBasicParsing -ErrorAction Stop
            Write-Log "Claude Code bootstrap script downloaded, executing in child process..."

            $bootstrapPath = Join-Path $Script:TempDir "claude-bootstrap.ps1"
            Set-Content -Path $bootstrapPath -Value $bootstrapScript -Encoding UTF8
            Unblock-File -Path $bootstrapPath -ErrorAction SilentlyContinue
            $bootstrapProcess = Start-Process -FilePath "powershell.exe" -ArgumentList "-ExecutionPolicy Bypass -File `"$bootstrapPath`"" -Wait -PassThru -NoNewWindow
            Write-Log "Claude Code bootstrap exit code: $($bootstrapProcess.ExitCode)"

            # Refresh PATH
            Refresh-PathFromRegistry

            # Ensure ~/.local/bin is in PATH
            $claudeBinDir = Join-Path $env:USERPROFILE ".local\bin"
            if (Test-Path $claudeBinDir) {
                Add-ToUserPath $claudeBinDir | Out-Null
                Refresh-PathFromRegistry
            }

            # Verify installation
            $claudeVersion = Get-CommandVersion -CommandName 'claude'
            if ($claudeVersion) {
                Write-Status "Claude Code installed successfully: v$claudeVersion" -Success
                $Script:Manifest.components.claudeCode.installed = $true
                $Script:Manifest.components.claudeCode.version = $claudeVersion
                $Script:Manifest.components.claudeCode.preExisting = $false
                return $true
            }

            # Fallback: try winget
            Write-Status "Trying winget as fallback..." -Warning
            try {
                $wingetCheck = Get-Command winget -ErrorAction Stop
                $wingetResult = & winget install Anthropic.ClaudeCode --accept-package-agreements --accept-source-agreements 2>&1
                Write-Log "winget install output: $wingetResult"

                Refresh-PathFromRegistry
                $claudeVersion = Get-CommandVersion -CommandName 'claude'
                if ($claudeVersion) {
                    Write-Status "Claude Code installed via winget: v$claudeVersion" -Success
                    $Script:Manifest.components.claudeCode.installed = $true
                    $Script:Manifest.components.claudeCode.version = $claudeVersion
                    $Script:Manifest.components.claudeCode.preExisting = $false
                    return $true
                }
            }
            catch {
                Write-Log "winget fallback failed: $_"
            }

            # If still not found, add PATH and warn about new terminal
            $claudeBinDir = Join-Path $env:USERPROFILE ".local\bin"
            Add-ToUserPath $claudeBinDir | Out-Null
            Write-Status "Claude Code may have been installed. Please open a NEW terminal and type 'claude' to verify." -Warning
            $Script:Manifest.components.claudeCode.installed = $true
            $Script:Manifest.components.claudeCode.version = 'unknown'
            $Script:Manifest.components.claudeCode.preExisting = $false
            return $true
        }
        catch {
            Write-ErrorMessage "Failed to install Claude Code: $($_.Exception.Message)" `
                "Please install manually by visiting: https://claude.ai/download"
            Write-Host "       Or run in PowerShell: irm https://claude.ai/install.ps1 | iex" -ForegroundColor White
            Write-Log "Claude Code installation FAILED: $_"
            return $false
        }
    }

    # ═══════════════════════════════════════════════════════════════════════════
    # PHASE 4: FINAL VERIFICATION & SUMMARY
    # ═══════════════════════════════════════════════════════════════════════════

    function Show-InstallSummary {
        Write-Step -Number 5 -Total 6 -Message "Verifying Claude Code works..."

        # Final PATH refresh
        Refresh-PathFromRegistry
        Send-SettingChange

        # Verify all components
        $gitVer = Get-CommandVersion -CommandName 'git'
        $nodeVer = Get-CommandVersion -CommandName 'node'
        $npmVer = Get-CommandVersion -CommandName 'npm'
        $claudeVer = Get-CommandVersion -CommandName 'claude'

        # Update manifest with final state
        if ($gitVer) { $Script:Manifest.components.git.version = $gitVer; $Script:Manifest.components.git.installed = $true }
        if ($nodeVer) { $Script:Manifest.components.node.version = $nodeVer; $Script:Manifest.components.node.installed = $true }
        if ($npmVer) { $Script:Manifest.components.npm.version = $npmVer; $Script:Manifest.components.npm.installed = $true }
        if ($claudeVer) { $Script:Manifest.components.claudeCode.version = $claudeVer; $Script:Manifest.components.claudeCode.installed = $true }

        # Save manifest
        Save-Manifest

        # Build summary
        $gitStatus = if ($gitVer) { "  [OK]  v$gitVer" } else { "  [XX]  NOT FOUND" }
        $nodeStatus = if ($nodeVer) { "  [OK]  v$nodeVer" } else { "  [XX]  NOT FOUND" }
        $npmStatus = if ($npmVer) { "  [OK]  v$npmVer" } else { "  [XX]  NOT FOUND" }
        $claudeStatus = if ($claudeVer) { "  [OK]  v$claudeVer" } else { "  [XX]  NOT FOUND" }

        $gitColor = if ($gitVer) { "Green" } else { "Red" }
        $nodeColor = if ($nodeVer) { "Green" } else { "Red" }
        $npmColor = if ($npmVer) { "Green" } else { "Red" }
        $claudeColor = if ($claudeVer) { "Green" } else { "Red" }

        $allOk = $gitVer -and $claudeVer  # Node/npm may be skipped

        Write-Host ""
        Write-Host "  ============================================================" -ForegroundColor Cyan
        Write-Host "         Claude Code Installation Summary" -ForegroundColor Cyan
        Write-Host "  ============================================================" -ForegroundColor Cyan
        Write-Host "  Git for Windows  " -NoNewline; Write-Host $gitStatus -ForegroundColor $gitColor
        Write-Host "  Node.js          " -NoNewline; Write-Host $nodeStatus -ForegroundColor $nodeColor
        Write-Host "  npm              " -NoNewline; Write-Host $npmStatus -ForegroundColor $npmColor
        Write-Host "  Claude Code      " -NoNewline; Write-Host $claudeStatus -ForegroundColor $claudeColor
        Write-Host "  ============================================================" -ForegroundColor Cyan

        if ($allOk) {
            Write-Host ""
            Write-Host "  All components installed successfully!" -ForegroundColor Green
            Write-Host ""
        }
        else {
            Write-Host ""

            if (-not $gitVer) {
                Write-Host "  [!!] Git: Install manually from https://git-scm.com/download/win" -ForegroundColor Yellow
            }
            if (-not $nodeVer -and -not $SkipNode) {
                Write-Host "  [!!] Node.js: Install manually from https://nodejs.org/en/download/" -ForegroundColor Yellow
            }
            if (-not $claudeVer) {
                Write-Host "  [!!] Claude Code: Open a NEW terminal and run:" -ForegroundColor Yellow
                Write-Host "       irm https://claude.ai/install.ps1 | iex" -ForegroundColor White
            }
            Write-Host ""
        }

        Write-Host "  Log file: $($Script:LogFile)" -ForegroundColor DarkGray
        Write-Host "  To uninstall, save this script and run: .\install-claude-code.ps1 -Uninstall" -ForegroundColor DarkGray
        Write-Host ""

        Write-Log "=== Installation complete ==="
        Write-Log "Git: $(if ($gitVer) { "v$gitVer" } else { 'NOT FOUND' })"
        Write-Log "Node: $(if ($nodeVer) { "v$nodeVer" } else { 'NOT FOUND' })"
        Write-Log "npm: $(if ($npmVer) { "v$npmVer" } else { 'NOT FOUND' })"
        Write-Log "Claude Code: $(if ($claudeVer) { "v$claudeVer" } else { 'NOT FOUND' })"
    }

    # ═══════════════════════════════════════════════════════════════════════════
    # PHASE 5/6: POST-INSTALL TEST, AUTO-REPAIR & LAUNCH
    # ═══════════════════════════════════════════════════════════════════════════

    function Test-ClaudeCodeRuntime {
        # Tests whether `claude --version` works and categorizes any failure.
        # Returns a hashtable with Success, Version, Error, BinaryPath, RawOutput.

        Refresh-PathFromRegistry

        # Candidate binary paths
        $candidates = @(
            (Join-Path $env:USERPROFILE ".local\bin\claude.exe"),
            (Join-Path $env:USERPROFILE ".local\bin\claude.cmd"),
            (Join-Path $env:USERPROFILE ".local\bin\claude"),
            (Join-Path $env:LOCALAPPDATA "Programs\claude\claude.exe")
        )

        # Try to find via PATH first
        $cmdPath = Get-CommandPath -CommandName 'claude'
        if (-not $cmdPath) {
            # Fall back to checking candidate paths directly
            $cmdPath = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
        }

        if (-not $cmdPath) {
            Write-Log "Test-ClaudeCodeRuntime: binary not found in PATH or candidate locations"
            return @{
                Success    = $false
                Version    = $null
                Error      = 'COMMAND_NOT_FOUND'
                BinaryPath = $null
                RawOutput  = 'claude binary not found on disk'
            }
        }

        Write-Log "Test-ClaudeCodeRuntime: using binary at $cmdPath"

        # Attempt to run
        try {
            $rawOutput = & $cmdPath --version 2>&1 | Out-String
            $exitCode = $LASTEXITCODE
            Write-Log "Test-ClaudeCodeRuntime: exit=$exitCode output=$rawOutput"

            $outputText = "$rawOutput".Trim()

            # Success path
            if ($exitCode -eq 0 -and $outputText) {
                $versionMatch = [regex]::Match($outputText, '(\d+\.\d+[\.\d]*)')
                $version = if ($versionMatch.Success) { $versionMatch.Groups[1].Value } else { $outputText }
                return @{
                    Success    = $true
                    Version    = $version
                    Error      = 'OK'
                    BinaryPath = $cmdPath
                    RawOutput  = $outputText
                }
            }

            # Classify failures by output patterns
            if ($outputText -match 'requires git-bash') {
                $errorKind = 'GIT_BASH_MISSING'
            }
            elseif ($outputText -match 'not recognized|CommandNotFound|cannot find') {
                $errorKind = 'COMMAND_NOT_FOUND'
            }
            elseif ($outputText -match 'Access.*denied|UnauthorizedAccess|Win32Exception|virus|blocked') {
                $errorKind = 'BLOCKED'
            }
            elseif (-not $outputText -and $exitCode -ne 0) {
                # Empty output + non-zero exit often indicates SmartScreen or silent block
                $errorKind = 'BLOCKED'
            }
            else {
                $errorKind = 'EXEC_FAILED'
            }

            return @{
                Success    = $false
                Version    = $null
                Error      = $errorKind
                BinaryPath = $cmdPath
                RawOutput  = $outputText
            }
        }
        catch {
            $errMsg = $_.Exception.Message
            Write-Log "Test-ClaudeCodeRuntime: exception=$errMsg"

            if ($errMsg -match 'Win32Exception|Access.*denied|UnauthorizedAccess|blocked|virus') {
                $errorKind = 'BLOCKED'
            }
            elseif ($errMsg -match 'not recognized|CommandNotFound|cannot find') {
                $errorKind = 'COMMAND_NOT_FOUND'
            }
            else {
                $errorKind = 'EXEC_FAILED'
            }

            return @{
                Success    = $false
                Version    = $null
                Error      = $errorKind
                BinaryPath = $cmdPath
                RawOutput  = $errMsg
            }
        }
    }

    function Repair-ClaudeCodeRuntime {
        param([hashtable]$InitialResult)

        $result = $InitialResult
        $maxAttempts = 3

        for ($attempt = 1; $attempt -le $maxAttempts; $attempt++) {
            if ($result.Success) { return $result }

            Write-Status "Auto-repair attempt $attempt/$maxAttempts — issue: $($result.Error)" -Warning
            Write-Log "Repair attempt $attempt for error: $($result.Error)"

            switch ($result.Error) {
                'COMMAND_NOT_FOUND' {
                    # Add ~/.local/bin to PATH
                    $claudeBinDir = Join-Path $env:USERPROFILE ".local\bin"
                    if (Test-Path $claudeBinDir) {
                        $added = Add-ToUserPath $claudeBinDir
                        if ($added) {
                            Write-Status "Added ~\.local\bin to User PATH" -Success
                        }
                        Refresh-PathFromRegistry
                    }
                    else {
                        # Binary dir doesn't exist — try re-running the bootstrap
                        Write-Status "Binary directory missing; re-running Claude Code installer..." -Info
                        try {
                            $bootstrapScript = Invoke-RestMethod -Uri 'https://claude.ai/install.ps1' -UseBasicParsing -ErrorAction Stop
                            $bootstrapPath = Join-Path $Script:TempDir "claude-bootstrap-retry.ps1"
                            Set-Content -Path $bootstrapPath -Value $bootstrapScript -Encoding UTF8
                            Unblock-File -Path $bootstrapPath -ErrorAction SilentlyContinue
                            Start-Process -FilePath "powershell.exe" `
                                -ArgumentList "-ExecutionPolicy Bypass -File `"$bootstrapPath`"" `
                                -Wait -NoNewWindow
                            Refresh-PathFromRegistry
                            if (Test-Path $claudeBinDir) {
                                Add-ToUserPath $claudeBinDir | Out-Null
                                Refresh-PathFromRegistry
                            }
                        }
                        catch {
                            Write-Log "Bootstrap retry failed: $_"
                        }
                    }
                }

                'GIT_BASH_MISSING' {
                    # Locate bash.exe
                    $bashCandidates = @(
                        (Join-Path $env:LOCALAPPDATA "Programs\Git\bin\bash.exe"),
                        (Join-Path $env:ProgramFiles "Git\bin\bash.exe"),
                        (Join-Path ${env:ProgramFiles(x86)} "Git\bin\bash.exe")
                    )
                    $bashPath = $bashCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1

                    if ($bashPath) {
                        [Environment]::SetEnvironmentVariable('CLAUDE_CODE_GIT_BASH_PATH', $bashPath, 'User')
                        $env:CLAUDE_CODE_GIT_BASH_PATH = $bashPath
                        Write-Status "Set CLAUDE_CODE_GIT_BASH_PATH=$bashPath" -Success
                        Write-Log "Set CLAUDE_CODE_GIT_BASH_PATH to $bashPath"
                    }
                    else {
                        Write-Status "bash.exe not found — re-installing Git..." -Warning
                        Install-GitComponent | Out-Null
                        # Re-check bash after Git install
                        $bashPath = $bashCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
                        if ($bashPath) {
                            [Environment]::SetEnvironmentVariable('CLAUDE_CODE_GIT_BASH_PATH', $bashPath, 'User')
                            $env:CLAUDE_CODE_GIT_BASH_PATH = $bashPath
                        }
                    }
                }

                'BLOCKED' {
                    Write-Status "Unblocking Claude Code files..." -Info
                    # Unblock the specific binary
                    if ($result.BinaryPath -and (Test-Path $result.BinaryPath)) {
                        Unblock-File -Path $result.BinaryPath -ErrorAction SilentlyContinue
                    }
                    # Unblock everything under ~/.local/bin
                    $localBin = Join-Path $env:USERPROFILE ".local\bin"
                    if (Test-Path $localBin) {
                        Get-ChildItem -Path $localBin -File -ErrorAction SilentlyContinue |
                            ForEach-Object { Unblock-File -Path $_.FullName -ErrorAction SilentlyContinue }
                    }
                    # Unblock files under ~/.local/share/claude
                    $localShare = Join-Path $env:USERPROFILE ".local\share\claude"
                    if (Test-Path $localShare) {
                        Get-ChildItem -Path $localShare -File -Recurse -ErrorAction SilentlyContinue |
                            ForEach-Object { Unblock-File -Path $_.FullName -ErrorAction SilentlyContinue }
                    }
                    Write-Status "Files unblocked" -Success
                }

                default {
                    # EXEC_FAILED / UNKNOWN — no automatic fix, stop looping
                    Write-Status "Unknown error — cannot auto-repair" -Error
                    Write-Log "Unfixable error. Raw output: $($result.RawOutput)"
                    return $result
                }
            }

            # Re-test
            Refresh-PathFromRegistry
            $result = Test-ClaudeCodeRuntime
        }

        return $result
    }

    function Start-ClaudeCodeLaunch {
        Write-Step -Number 6 -Total 6 -Message "Launching Claude Code in a new terminal..."

        if ($Script:DryRunMode) {
            Write-Status "[DRY RUN] Would open a new PowerShell window and run 'claude'" -Info
            return
        }

        # Ensure current session env is fresh so child inherits correct PATH
        Refresh-PathFromRegistry

        # Propagate CLAUDE_CODE_GIT_BASH_PATH from registry if set
        $gitBashPath = [Environment]::GetEnvironmentVariable('CLAUDE_CODE_GIT_BASH_PATH', 'User')
        if ($gitBashPath) {
            $env:CLAUDE_CODE_GIT_BASH_PATH = $gitBashPath
        }

        try {
            # Write launch wrapper script
            $launchScript = @'
$host.UI.RawUI.WindowTitle = 'Claude Code'
Write-Host ''
Write-Host '  ============================================================' -ForegroundColor Cyan
Write-Host '    Claude Code is ready!' -ForegroundColor Green
Write-Host '    Starting now... (you can close this window anytime)' -ForegroundColor Cyan
Write-Host '  ============================================================' -ForegroundColor Cyan
Write-Host ''
Start-Sleep -Seconds 1
try {
    claude
}
catch {
    Write-Host ''
    Write-Host '  [ERROR] Failed to start Claude Code:' -ForegroundColor Red
    Write-Host "  $($_.Exception.Message)" -ForegroundColor Yellow
    Write-Host ''
    Write-Host '  Try running this command manually:  claude' -ForegroundColor White
    Write-Host ''
}
'@

            $launchScriptPath = Join-Path $Script:TempDir "launch-claude.ps1"
            Set-Content -Path $launchScriptPath -Value $launchScript -Encoding UTF8
            Write-Log "Wrote launch wrapper to $launchScriptPath"

            Write-Status "Opening new terminal window..." -Info
            Start-Process -FilePath "powershell.exe" -ArgumentList @(
                '-NoExit',
                '-NoProfile',
                '-ExecutionPolicy', 'Bypass',
                '-File', $launchScriptPath
            ) -WorkingDirectory $env:USERPROFILE | Out-Null

            Write-Status "Claude Code is now running in a new window!" -Success
            Write-Host ""
            Write-Host "  A new PowerShell window has opened with Claude Code running." -ForegroundColor Green
            Write-Host "  You can close this installer window." -ForegroundColor Cyan
            Write-Host ""
            Write-Log "Launched Claude Code in new terminal window"
        }
        catch {
            Write-ErrorMessage "Could not auto-launch Claude Code: $($_.Exception.Message)" `
                "Please open a new PowerShell window manually and type: claude"
            Write-Log "Launch failed: $_"
        }
    }

    # ═══════════════════════════════════════════════════════════════════════════
    # MANIFEST MANAGEMENT
    # ═══════════════════════════════════════════════════════════════════════════

    function Save-Manifest {
        if ($Script:DryRunMode) {
            Write-Log "[DRY RUN] Would save manifest to $($Script:ManifestPath)"
            return
        }
        try {
            $manifestDir = Split-Path $Script:ManifestPath -Parent
            if (-not (Test-Path $manifestDir)) {
                New-Item -ItemType Directory -Path $manifestDir -Force | Out-Null
            }
            $Script:Manifest | ConvertTo-Json -Depth 5 | Set-Content -Path $Script:ManifestPath -Encoding UTF8
            Write-Log "Manifest saved to $($Script:ManifestPath)"
        }
        catch {
            Write-Log "Failed to save manifest: $_"
        }
    }

    function Read-Manifest {
        if (Test-Path $Script:ManifestPath) {
            try {
                $content = Get-Content -Path $Script:ManifestPath -Raw | ConvertFrom-Json
                Write-Log "Manifest loaded from $($Script:ManifestPath)"
                return $content
            }
            catch {
                Write-Log "Failed to read manifest: $_"
                return $null
            }
        }
        return $null
    }

    # ═══════════════════════════════════════════════════════════════════════════
    # PHASE 5: UNINSTALL FLOW
    # ═══════════════════════════════════════════════════════════════════════════

    function Invoke-Uninstall {
        Write-Host ""
        Write-Host "  ============================================================" -ForegroundColor Cyan
        Write-Host "         Claude Code Uninstaller" -ForegroundColor Cyan
        Write-Host "  ============================================================" -ForegroundColor Cyan
        Write-Host ""
        Write-Host "  What would you like to uninstall?" -ForegroundColor White
        Write-Host ""
        Write-Host "  [1] Claude Code only                  " -NoNewline -ForegroundColor White
        Write-Host "(recommended)" -ForegroundColor Green
        Write-Host "  [2] Claude Code + Node.js" -ForegroundColor White
        Write-Host "  [3] Claude Code + Node.js + Git" -ForegroundColor White
        Write-Host "  [4] Everything this installer installed" -ForegroundColor White
        Write-Host "  [0] Cancel" -ForegroundColor DarkGray
        Write-Host ""
        Write-Host "  ============================================================" -ForegroundColor Cyan
        Write-Host ""

        if ($Script:NonInteractive) {
            Write-ErrorMessage "Uninstall requires interactive mode." `
                "Save the script and run: .\install-claude-code.ps1 -Uninstall"
            return
        }

        $choice = Read-Host "  Enter your choice [0-4]"
        Write-Log "Uninstall choice: $choice"

        if ($choice -eq '0' -or [string]::IsNullOrWhiteSpace($choice)) {
            Write-Status "Uninstall cancelled." -Info
            return
        }

        # Load manifest to check what we installed
        $manifest = Read-Manifest

        $removeClaude = $choice -in @('1', '2', '3', '4')
        $removeNode = $choice -in @('2', '3', '4')
        $removeGit = $choice -in @('3', '4')

        $results = @{}

        # Uninstall Claude Code
        if ($removeClaude) {
            Write-Host ""
            Write-Status "Uninstalling Claude Code..." -Info
            $results['Claude Code'] = Uninstall-ClaudeCode
        }

        # Uninstall Node.js
        if ($removeNode) {
            $preExisting = $false
            if ($manifest -and $manifest.components.node.preExisting) {
                $preExisting = $true
            }

            if ($preExisting) {
                Write-Host ""
                Write-Status "Node.js was already installed before this installer ran." -Warning
                $answer = Read-UserChoice -Prompt "Are you sure you want to remove it? [y/N]" -Default 'N'
                if ($answer -ne 'Y') {
                    Write-Status "Skipping Node.js removal" -Info
                    $results['Node.js'] = 'skipped'
                }
                else {
                    $results['Node.js'] = Uninstall-Node
                }
            }
            else {
                $results['Node.js'] = Uninstall-Node
            }
        }

        # Uninstall Git
        if ($removeGit) {
            Write-Host ""
            Write-Host "  ============================================================" -ForegroundColor Yellow
            Write-Host "  WARNING: Git may be used by other programs such as:" -ForegroundColor Yellow
            Write-Host "    - VS Code" -ForegroundColor Yellow
            Write-Host "    - GitHub Desktop" -ForegroundColor Yellow
            Write-Host "    - SourceTree" -ForegroundColor Yellow
            Write-Host "    - Other development tools" -ForegroundColor Yellow
            Write-Host "  ============================================================" -ForegroundColor Yellow

            $answer = Read-UserChoice -Prompt "Are you SURE you want to remove Git for Windows? [y/N]" -Default 'N'
            if ($answer -ne 'Y') {
                Write-Status "Skipping Git removal" -Info
                $results['Git'] = 'skipped'
            }
            else {
                $results['Git'] = Uninstall-Git
            }
        }

        # Cleanup
        Write-Host ""
        Write-Status "Cleaning up..." -Info

        # Remove temp files
        if (Test-Path $Script:TempDir) {
            Remove-Item -Path $Script:TempDir -Recurse -Force -ErrorAction SilentlyContinue
            Write-Log "Removed temp directory: $Script:TempDir"
        }

        # Remove log file
        if (Test-Path $Script:LogFile) {
            Remove-Item -Path $Script:LogFile -Force -ErrorAction SilentlyContinue
        }

        # Remove manifest if everything was removed
        if ($choice -eq '4') {
            $manifestDir = Split-Path $Script:ManifestPath -Parent
            if (Test-Path $manifestDir) {
                Remove-Item -Path $manifestDir -Recurse -Force -ErrorAction SilentlyContinue
                Write-Log "Removed manifest directory"
            }
        }

        # Broadcast PATH changes
        Send-SettingChange

        # Display results
        Write-Host ""
        Write-Host "  ============================================================" -ForegroundColor Cyan
        Write-Host "         Uninstall Results" -ForegroundColor Cyan
        Write-Host "  ============================================================" -ForegroundColor Cyan

        foreach ($component in $results.Keys) {
            $status = $results[$component]
            if ($status -eq $true -or $status -eq 'ok') {
                Write-Host "  $component" -NoNewline; Write-Host "  [OK] Removed" -ForegroundColor Green
            }
            elseif ($status -eq 'skipped') {
                Write-Host "  $component" -NoNewline; Write-Host "  [--] Skipped" -ForegroundColor DarkGray
            }
            else {
                Write-Host "  $component" -NoNewline; Write-Host "  [XX] Failed: $status" -ForegroundColor Red
            }
        }

        Write-Host "  ============================================================" -ForegroundColor Cyan
        Write-Host ""
        Write-Host "  Open a NEW terminal window for changes to take effect." -ForegroundColor Yellow
        Write-Host ""
    }

    function Uninstall-ClaudeCode {
        Write-Log "Uninstalling Claude Code..."
        try {
            # Remove binary files
            $claudeBin = Join-Path $env:USERPROFILE ".local\bin"
            $claudeFiles = Get-ChildItem -Path $claudeBin -Filter "claude*" -ErrorAction SilentlyContinue
            if ($claudeFiles) {
                $claudeFiles | Remove-Item -Force -ErrorAction SilentlyContinue
                Write-Status "Removed Claude Code binary" -Success
            }

            # Remove version/share data
            $claudeShareDir = Join-Path $env:USERPROFILE ".local\share\claude"
            if (Test-Path $claudeShareDir) {
                Remove-Item -Path $claudeShareDir -Recurse -Force -ErrorAction SilentlyContinue
                Write-Status "Removed Claude Code data" -Success
            }

            # Try winget uninstall as fallback
            try {
                $null = Get-Command winget -ErrorAction Stop
                & winget uninstall Anthropic.ClaudeCode --silent 2>&1 | Out-Null
                Write-Log "winget uninstall attempted"
            }
            catch { }

            # Ask about config files
            $claudeConfigDir = Join-Path $env:USERPROFILE ".claude"
            $claudeConfigJson = Join-Path $env:USERPROFILE ".claude.json"
            $hasConfig = (Test-Path $claudeConfigDir) -or (Test-Path $claudeConfigJson)

            if ($hasConfig) {
                Write-Host ""
                $answer = Read-UserChoice -Prompt "Remove Claude Code config files (~\.claude, ~\.claude.json)? [y/N]" -Default 'N'
                if ($answer -eq 'Y') {
                    if (Test-Path $claudeConfigDir) {
                        Remove-Item -Path $claudeConfigDir -Recurse -Force -ErrorAction SilentlyContinue
                    }
                    if (Test-Path $claudeConfigJson) {
                        Remove-Item -Path $claudeConfigJson -Force -ErrorAction SilentlyContinue
                    }
                    Write-Status "Removed Claude Code config files" -Success
                }
                else {
                    Write-Status "Config files preserved for future reinstall" -Info
                }
            }

            # Remove PATH entry if .local\bin is empty
            $claudeBinDir = Join-Path $env:USERPROFILE ".local\bin"
            if (Test-Path $claudeBinDir) {
                $remaining = Get-ChildItem -Path $claudeBinDir -ErrorAction SilentlyContinue
                if (-not $remaining -or $remaining.Count -eq 0) {
                    Remove-FromUserPath $claudeBinDir | Out-Null
                    Remove-Item -Path $claudeBinDir -Force -ErrorAction SilentlyContinue
                }
            }

            Write-Log "Claude Code uninstalled"
            return $true
        }
        catch {
            Write-Log "Claude Code uninstall failed: $_"
            return "Error: $($_.Exception.Message)"
        }
    }

    function Uninstall-Node {
        Write-Log "Uninstalling Node.js..."
        try {
            # Try to find MSI product code from registry
            $nodeUninstall = Get-ItemProperty "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*" -ErrorAction SilentlyContinue |
                Where-Object { $_.DisplayName -match 'Node\.js' } |
                Select-Object -First 1

            if ($nodeUninstall -and $nodeUninstall.PSChildName) {
                $productCode = $nodeUninstall.PSChildName
                Write-Status "Uninstalling Node.js (MSI: $productCode)..." -Info
                $process = Start-Process -FilePath "msiexec.exe" -ArgumentList "/x $productCode /quiet /norestart" -Wait -PassThru -NoNewWindow
                Write-Log "Node.js MSI uninstall exit code: $($process.ExitCode)"
                if ($process.ExitCode -eq 0 -or $process.ExitCode -eq 3010) {
                    Write-Status "Node.js uninstalled via MSI" -Success
                }
            }
            else {
                # Try winget
                Write-Status "Trying winget to uninstall Node.js..." -Info
                try {
                    $null = Get-Command winget -ErrorAction Stop
                    & winget uninstall OpenJS.NodeJS.LTS --silent 2>&1 | Out-Null
                    Write-Status "Node.js uninstall attempted via winget" -Info
                }
                catch {
                    Write-Status "Could not find Node.js uninstaller. You may need to remove it manually via Settings > Apps." -Warning
                }
            }

            # Remove npm-global directory
            $npmGlobalDir = Join-Path $env:USERPROFILE ".npm-global"
            if (Test-Path $npmGlobalDir) {
                Remove-Item -Path $npmGlobalDir -Recurse -Force -ErrorAction SilentlyContinue
                Write-Status "Removed npm global directory" -Success
            }

            # Ask about npm cache
            $npmDir = Join-Path $env:APPDATA "npm"
            $npmCacheDir = Join-Path $env:APPDATA "npm-cache"
            $hasNpmData = (Test-Path $npmDir) -or (Test-Path $npmCacheDir)

            if ($hasNpmData) {
                $answer = Read-UserChoice -Prompt "Remove npm cache directories (AppData\Roaming\npm, npm-cache)? [y/N]" -Default 'N'
                if ($answer -eq 'Y') {
                    if (Test-Path $npmDir) { Remove-Item -Path $npmDir -Recurse -Force -ErrorAction SilentlyContinue }
                    if (Test-Path $npmCacheDir) { Remove-Item -Path $npmCacheDir -Recurse -Force -ErrorAction SilentlyContinue }
                    Write-Status "Removed npm cache directories" -Success
                }
            }

            # Remove PATH entries
            Remove-FromUserPath (Join-Path $env:ProgramFiles "nodejs") | Out-Null
            Remove-FromUserPath (Join-Path $env:USERPROFILE ".npm-global") | Out-Null
            Remove-FromUserPath (Join-Path $env:APPDATA "npm") | Out-Null

            Refresh-PathFromRegistry
            Write-Log "Node.js uninstalled"
            return $true
        }
        catch {
            Write-Log "Node.js uninstall failed: $_"
            return "Error: $($_.Exception.Message)"
        }
    }

    function Uninstall-Git {
        Write-Log "Uninstalling Git for Windows..."
        try {
            # Try to find the Git uninstaller
            $gitUninstallerPaths = @(
                (Join-Path $env:LOCALAPPDATA "Programs\Git\unins000.exe"),
                (Join-Path $env:ProgramFiles "Git\unins000.exe")
            )

            $gitUninstaller = $gitUninstallerPaths | Where-Object { Test-Path $_ } | Select-Object -First 1

            if ($gitUninstaller) {
                Write-Status "Uninstalling Git for Windows..." -Info
                $process = Start-Process -FilePath $gitUninstaller -ArgumentList "/VERYSILENT /NORESTART" -Wait -PassThru -NoNewWindow
                Write-Log "Git uninstaller exit code: $($process.ExitCode)"
                Write-Status "Git for Windows uninstalled" -Success
            }
            else {
                # Try winget
                Write-Status "Trying winget to uninstall Git..." -Info
                try {
                    $null = Get-Command winget -ErrorAction Stop
                    & winget uninstall Git.Git --silent 2>&1 | Out-Null
                    Write-Status "Git uninstall attempted via winget" -Info
                }
                catch {
                    Write-Status "Could not find Git uninstaller. You may need to remove it manually via Settings > Apps." -Warning
                }
            }

            # Remove PATH entries
            Remove-FromUserPath (Join-Path $env:LOCALAPPDATA "Programs\Git\cmd") | Out-Null
            Remove-FromUserPath (Join-Path $env:ProgramFiles "Git\cmd") | Out-Null

            Refresh-PathFromRegistry
            Write-Log "Git uninstalled"
            return $true
        }
        catch {
            Write-Log "Git uninstall failed: $_"
            return "Error: $($_.Exception.Message)"
        }
    }

    # ═══════════════════════════════════════════════════════════════════════════
    # ORCHESTRATORS
    # ═══════════════════════════════════════════════════════════════════════════

    function Invoke-Install {
        Show-Banner

        # Phase 0: Pre-flight
        $preflightOk = Invoke-PreflightChecks
        if (-not $preflightOk) {
            Write-Log "Installation aborted: pre-flight checks failed"
            return
        }

        # Phase 1: Git
        try {
            $gitOk = Install-GitComponent
        }
        catch {
            Write-ErrorMessage "Unexpected error during Git installation: $($_.Exception.Message)"
            Write-Log "FATAL Git error: $_"
            $gitOk = $false
        }

        # Phase 2: Node.js
        try {
            $nodeOk = Install-NodeComponent -Skip:$SkipNode
        }
        catch {
            Write-ErrorMessage "Unexpected error during Node.js installation: $($_.Exception.Message)"
            Write-Log "FATAL Node.js error: $_"
            $nodeOk = $false
        }

        # Phase 3: Claude Code
        try {
            $claudeOk = Install-ClaudeCodeComponent
        }
        catch {
            Write-ErrorMessage "Unexpected error during Claude Code installation: $($_.Exception.Message)"
            Write-Log "FATAL Claude Code error: $_"
            $claudeOk = $false
        }

        # Phase 4: Summary
        Show-InstallSummary

        # Phase 5: Test Claude Code runtime & auto-repair
        if ($Script:DryRunMode) {
            Write-Step -Number 5 -Total 6 -Message "Verifying Claude Code works..."
            Write-Status "[DRY RUN] Would test 'claude --version' and auto-repair any issues" -Info
            Start-ClaudeCodeLaunch
            return
        }

        try {
            $testResult = Test-ClaudeCodeRuntime
            if (-not $testResult.Success) {
                Write-Status "Claude Code test failed ($($testResult.Error)). Running auto-repair..." -Warning
                Write-Log "Test failed: $($testResult.Error) | $($testResult.RawOutput)"
                $testResult = Repair-ClaudeCodeRuntime -InitialResult $testResult
            }

            if ($testResult.Success) {
                Write-Status "Claude Code is working correctly: v$($testResult.Version)" -Success
                Write-Log "Claude Code runtime test PASSED after repair: v$($testResult.Version)"

                # Phase 6: Launch in a new terminal
                Start-ClaudeCodeLaunch
            }
            else {
                Write-ErrorMessage "Claude Code verification failed: $($testResult.Error)" `
                    "Please open a NEW PowerShell window and type 'claude' manually."
                Write-Host "       Error details: $($testResult.RawOutput)" -ForegroundColor Yellow
                Write-Host "       Full log: $($Script:LogFile)" -ForegroundColor DarkGray
                Write-Log "Claude Code runtime test FAILED after all repair attempts"
            }
        }
        catch {
            Write-ErrorMessage "Unexpected error during Claude Code verification: $($_.Exception.Message)" `
                "Please open a NEW PowerShell window and type 'claude' manually."
            Write-Log "FATAL test/launch error: $_"
        }
    }

    # ═══════════════════════════════════════════════════════════════════════════
    # MAIN ENTRY (inside function)
    # ═══════════════════════════════════════════════════════════════════════════

    if ($Uninstall) {
        Show-Banner
        Invoke-Uninstall
    }
    else {
        Invoke-Install
    }
}

# ═══════════════════════════════════════════════════════════════════════════════
# ENTRY POINT — Dual-mode dispatcher
# ═══════════════════════════════════════════════════════════════════════════════
# When piped via `irm ... | iex`, $PSCommandPath is null.
# When run as a saved file, $PSCommandPath is the script path.

$_isPiped = [string]::IsNullOrEmpty($PSCommandPath)

if ($_isPiped) {
    # Non-interactive mode: install with defaults (irm | iex)
    Invoke-ClaudeCodeInstaller
}
else {
    # Interactive mode: forward script-level params to the function
    Invoke-ClaudeCodeInstaller -Uninstall:$Uninstall -SkipNode:$SkipNode -DryRun:$DryRun
}

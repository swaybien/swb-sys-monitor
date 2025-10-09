# PowerShell script to setup pre-commit hooks

# Function to print colored log messages
function Write-LogInfo {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor Cyan
}

function Write-LogSuccess {
    param([string]$Message)
    Write-Host "[SUCCESS] $Message" -ForegroundColor Green
}

function Write-LogWarning {
    param([string]$Message)
    Write-Host "[WARNING] $Message" -ForegroundColor Yellow
}

function Write-LogError {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor Red
}

# Check if we're in a git repository
if (-not (Test-Path ".git")) {
    Write-LogError "Not in a git repository. Please run this script from the root of your git repository."
    exit 1
}

Write-LogInfo "Setting up pre-commit hooks..."

# Create .git/hooks directory if it doesn't exist
if (-not (Test-Path ".git/hooks")) {
    Write-LogInfo "Creating .git/hooks directory..."
    New-Item -ItemType Directory -Path ".git/hooks" -Force | Out-Null
}

# Pre-commit hook content (Windows batch version)
$PreCommitHookContent = @'
@echo off
setlocal enabledelayedexpansion

:: ANSI color codes for Windows 10+ Command Prompt
set "RED=[91m"
set "GREEN=[92m"
set "YELLOW=[93m"
set "BLUE=[94m"
set "CYAN=[96m"
set "NC=[0m"

echo %CYAN%[INFO]%NC% Running pre-commit checks...

:: Check if cargo is available
where cargo >nul 2>&1
if !errorlevel! neq 0 (
    echo %RED%[ERROR]%NC% Cargo is not installed or not in PATH
    exit /b 1
)

:: Run cargo fmt check
echo %CYAN%[INFO]%NC% Checking code formatting with cargo fmt...
cargo fmt --all -- --check >nul 2>&1
if !errorlevel! neq 0 (
    echo %RED%[ERROR]%NC% Code formatting check failed. Please run 'cargo fmt' to fix formatting issues.
    exit /b 1
)
echo %GREEN%[SUCCESS]%NC% Code formatting check passed

:: Run cargo clippy
echo %CYAN%[INFO]%NC% Running cargo clippy...
cargo clippy --all-targets --all-features -- -D warnings -A dead_code >nul 2>&1
if !errorlevel! neq 0 (
    echo %RED%[ERROR]%NC% Clippy check failed. Please fix the warnings and errors.
    exit /b 1
)
echo %GREEN%[SUCCESS]%NC% Clippy check passed

:: Run tests
echo %CYAN%[INFO]%NC% Running tests...
cargo test >nul 2>&1
if !errorlevel! neq 0 (
    echo %RED%[ERROR]%NC% Tests failed. Please fix the failing tests.
    exit /b 1
)
echo %GREEN%[SUCCESS]%NC% All tests passed

echo %GREEN%[SUCCESS]%NC% All pre-commit checks passed!
'@

# Write the pre-commit hook (both standard and .cmd versions for Windows compatibility)
Write-LogInfo "Writing pre-commit hook to .git/hooks/pre-commit..."
$PreCommitHookContent | Out-File -FilePath ".git/hooks/pre-commit" -Encoding ASCII
$PreCommitHookContent | Out-File -FilePath ".git/hooks/pre-commit.cmd" -Encoding ASCII

# Set executable permissions (Windows equivalent)
try {
    $acl = Get-Acl ".git/hooks/pre-commit"
    $accessRule = New-Object System.Security.AccessControl.FileSystemAccessRule("Everyone", "FullControl", "Allow")
    $acl.SetAccessRule($accessRule)
    Set-Acl ".git/hooks/pre-commit" $acl
    Set-Acl ".git/hooks/pre-commit.cmd" $acl
    Write-LogInfo "Set executable permissions on pre-commit hooks"
} catch {
    Write-LogWarning "Could not set executable permissions: $($_.Exception.Message)"
}

# Verify the installation
if ((Test-Path ".git/hooks/pre-commit") -and (Test-Path ".git/hooks/pre-commit.cmd")) {
    Write-LogSuccess "Pre-commit hook successfully installed!"
    Write-LogInfo "The hook will run automatically before each commit."
    Write-LogInfo "Created both 'pre-commit' and 'pre-commit.cmd' for maximum Windows compatibility."
    Write-LogInfo "To test the hook manually, run: .git/hooks/pre-commit.cmd"
} else {
    Write-LogError "Failed to install pre-commit hook"
    exit 1
}

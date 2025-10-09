@echo off
setlocal enabledelayedexpansion

:: ANSI color codes for Windows 10+ Command Prompt
set "RED=[91m"
set "GREEN=[92m"
set "YELLOW=[93m"
set "BLUE=[94m"
set "CYAN=[96m"
set "NC=[0m"

:: Function to print colored log messages
goto :main

:log_info
echo %CYAN%[INFO]%NC% %~1
goto :eof

:log_success
echo %GREEN%[SUCCESS]%NC% %~1
goto :eof

:log_warning
echo %YELLOW%[WARNING]%NC% %~1
goto :eof

:log_error
echo %RED%[ERROR]%NC% %~1
goto :eof

:main
:: Check if we're in a git repository
if not exist ".git" (
    call :log_error "Not in a git repository. Please run this script from the root of your git repository."
    exit /b 1
)

call :log_info "Setting up pre-commit hooks..."

:: Create .git/hooks directory if it doesn't exist
if not exist ".git\hooks" (
    call :log_info "Creating .git/hooks directory..."
    mkdir ".git\hooks"
)

:: Create the pre-commit hook content
call :log_info "Writing pre-commit hook to .git/hooks/pre-commit..."

(
echo @echo off
echo setlocal enabledelayedexpansion
echo.
echo :: ANSI color codes for Windows 10+ Command Prompt
echo set "RED=[91m"
echo set "GREEN=[92m"
echo set "YELLOW=[93m"
echo set "BLUE=[94m"
echo set "CYAN=[96m"
echo set "NC=[0m"
echo.
echo echo %%CYAN%%[INFO]%%NC%% Running pre-commit checks...
echo.
echo :: Check if cargo is available
echo where cargo ^>nul 2^>^&1
echo if ^^!errorlevel^^! neq 0 ^(
echo     echo %%RED%%[ERROR]%%NC%% Cargo is not installed or not in PATH
echo     exit /b 1
echo ^)
echo.
echo :: Run cargo fmt check
echo echo %%CYAN%%[INFO]%%NC%% Checking code formatting with cargo fmt...
echo cargo fmt --all -- --check ^>nul 2^>^&1
echo if ^^!errorlevel^^! neq 0 ^(
echo     echo %%RED%%[ERROR]%%NC%% Code formatting check failed. Please run 'cargo fmt' to fix formatting issues.
echo     exit /b 1
echo ^)
echo echo %%GREEN%%[SUCCESS]%%NC%% Code formatting check passed
echo.
echo :: Run cargo clippy
echo echo %%CYAN%%[INFO]%%NC%% Running cargo clippy...
echo cargo clippy --all-targets --all-features -- -D warnings -A dead_code ^>nul 2^>^&1
echo if ^^!errorlevel^^! neq 0 ^(
echo     echo %%RED%%[ERROR]%%NC%% Clippy check failed. Please fix the warnings and errors.
echo     exit /b 1
echo ^)
echo echo %%GREEN%%[SUCCESS]%%NC%% Clippy check passed
echo.
echo :: Run tests
echo echo %%CYAN%%[INFO]%%NC%% Running tests...
echo cargo test ^>nul 2^>^&1
echo if ^^!errorlevel^^! neq 0 ^(
echo     echo %%RED%%[ERROR]%%NC%% Tests failed. Please fix the failing tests.
echo     exit /b 1
echo ^)
echo echo %%GREEN%%[SUCCESS]%%NC%% All tests passed
echo.
echo echo %%GREEN%%[SUCCESS]%%NC%% All pre-commit checks passed!
) > ".git\hooks\pre-commit"

:: Also create .cmd version for Windows compatibility
copy ".git\hooks\pre-commit" ".git\hooks\pre-commit.cmd" >nul

:: Set executable permissions
call :log_info "Setting executable permissions on pre-commit hooks..."
icacls ".git\hooks\pre-commit" /grant Everyone:F >nul 2>&1
icacls ".git\hooks\pre-commit.cmd" /grant Everyone:F >nul 2>&1
if !errorlevel! equ 0 (
    call :log_info "Executable permissions set successfully"
) else (
    call :log_warning "Could not set executable permissions"
)

:: Verify the installation
if exist ".git\hooks\pre-commit" (
    if exist ".git\hooks\pre-commit.cmd" (
        call :log_success "Pre-commit hook successfully installed!"
        call :log_info "The hook will run automatically before each commit."
        call :log_info "Created both 'pre-commit' and 'pre-commit.cmd' for maximum Windows compatibility."
        call :log_info "To test the hook manually, run: .git/hooks/pre-commit.cmd"
    ) else (
        call :log_error "Failed to create .cmd version of pre-commit hook"
        exit /b 1
    )
) else (
    call :log_error "Failed to install pre-commit hook"
    exit /b 1
)

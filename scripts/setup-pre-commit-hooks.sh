#!/bin/bash

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Function to print colored log messages
log_info() {
    echo -e "${CYAN}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if we're in a git repository
if [ ! -d ".git" ]; then
    log_error "Not in a git repository. Please run this script from the root of your git repository."
    exit 1
fi

log_info "Setting up pre-commit hooks..."

# Create .git/hooks directory if it doesn't exist
if [ ! -d ".git/hooks" ]; then
    log_info "Creating .git/hooks directory..."
    mkdir -p .git/hooks
fi

# Create the pre-commit hook file
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${CYAN}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_info "Running pre-commit checks..."

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    log_error "Cargo is not installed or not in PATH"
    exit 1
fi

# Run cargo fmt check
log_info "Checking code formatting with cargo fmt..."
if ! cargo fmt --all -- --check; then
    log_error "Code formatting check failed. Please run '\''cargo fmt'\'' to fix formatting issues."
    exit 1
fi
log_success "Code formatting check passed"

# Run cargo clippy
log_info "Running cargo clippy..."
if ! cargo clippy --all-targets --all-features -- -D warnings -A dead_code; then
    log_error "Clippy check failed. Please fix the warnings and errors."
    exit 1
fi
log_success "Clippy check passed"

# Run tests
log_info "Running tests..."
if ! cargo test; then
    log_error "Tests failed. Please fix the failing tests."
    exit 1
fi
log_success "All tests passed"

log_success "All pre-commit checks passed!"
EOF

# Write the pre-commit hook
log_info "Writing pre-commit hook to .git/hooks/pre-commit..."

# Make the hook executable
chmod +x .git/hooks/pre-commit
log_success "Pre-commit hook installed and made executable"

# Verify the installation
if [ -f ".git/hooks/pre-commit" ] && [ -x ".git/hooks/pre-commit" ]; then
    log_success "Pre-commit hook successfully installed!"
    log_info "The hook will run automatically before each commit."
    log_info "To test the hook manually, run: .git/hooks/pre-commit"
else
    log_error "Failed to install pre-commit hook"
    exit 1
fi

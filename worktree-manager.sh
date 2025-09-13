#!/bin/bash

# Git Worktree Manager for RT Terminal Emulator Project
# This script helps manage git worktrees for parallel development streams

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKTREES_DIR="$SCRIPT_DIR/worktrees"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to show usage
show_usage() {
    echo "Git Worktree Manager for RT Terminal Emulator"
    echo ""
    echo "Usage: $0 <command> [options]"
    echo ""
    echo "Commands:"
    echo "  setup                    Set up all worktrees"
    echo "  list                     List all worktrees"
    echo "  add <name> <branch>      Add a new worktree"
    echo "  remove <name>            Remove a worktree"
    echo "  clean                    Remove all worktrees"
    echo "  status                   Show status of all worktrees"
    echo "  sync <name>              Sync worktree with main branch"
    echo "  test                     Test worktree isolation"
    echo "  help                     Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 setup                 # Set up all worktrees"
    echo "  $0 list                  # List all worktrees"
    echo "  $0 add feature/new-feature feature/new-feature"
    echo "  $0 remove rendering-optimization"
    echo "  $0 sync rendering-optimization"
    echo ""
}

# Function to set up all worktrees
setup_worktrees() {
    print_status "Setting up worktrees..."
    
    # Create worktrees directory if it doesn't exist
    mkdir -p "$WORKTREES_DIR"
    
    # Define worktrees to create
    declare -A worktrees=(
        ["rendering-optimization"]="feature/rendering-optimization"
        ["ui-enhancements"]="feature/ui-enhancements"
        ["security-compliance"]="feature/security-compliance"
    )
    
    # Create branches if they don't exist
    for branch in "${worktrees[@]}"; do
        if ! git show-ref --verify --quiet "refs/heads/$branch"; then
            print_status "Creating branch: $branch"
            git checkout -b "$branch" main
            git checkout main
        fi
    done
    
    # Create worktrees
    for name in "${!worktrees[@]}"; do
        branch="${worktrees[$name]}"
        worktree_path="$WORKTREES_DIR/$name"
        
        if [ -d "$worktree_path" ]; then
            print_warning "Worktree '$name' already exists"
        else
            print_status "Creating worktree: $name -> $branch"
            git worktree add "$worktree_path" "$branch"
            print_success "Worktree '$name' created successfully"
        fi
    done
    
    print_success "All worktrees set up successfully"
}

# Function to list all worktrees
list_worktrees() {
    print_status "Listing all worktrees:"
    echo ""
    git worktree list
    echo ""
    
    # Show additional information
    if [ -d "$WORKTREES_DIR" ]; then
        echo "Worktree directory structure:"
        find "$WORKTREES_DIR" -maxdepth 1 -type d -name "*" | while read -r dir; do
            if [ "$dir" != "$WORKTREES_DIR" ]; then
                name=$(basename "$dir")
                if [ -f "$dir/.git" ]; then
                    branch=$(cd "$dir" && git branch --show-current)
                    echo "  $name -> $branch"
                fi
            fi
        done
    fi
}

# Function to add a new worktree
add_worktree() {
    if [ $# -ne 2 ]; then
        print_error "Usage: $0 add <name> <branch>"
        exit 1
    fi
    
    name="$1"
    branch="$2"
    worktree_path="$WORKTREES_DIR/$name"
    
    if [ -d "$worktree_path" ]; then
        print_error "Worktree '$name' already exists"
        exit 1
    fi
    
    # Create branch if it doesn't exist
    if ! git show-ref --verify --quiet "refs/heads/$branch"; then
        print_status "Creating branch: $branch"
        git checkout -b "$branch" main
        git checkout main
    fi
    
    # Create worktrees directory if it doesn't exist
    mkdir -p "$WORKTREES_DIR"
    
    print_status "Adding worktree: $name -> $branch"
    git worktree add "$worktree_path" "$branch"
    print_success "Worktree '$name' added successfully"
}

# Function to remove a worktree
remove_worktree() {
    if [ $# -ne 1 ]; then
        print_error "Usage: $0 remove <name>"
        exit 1
    fi
    
    name="$1"
    worktree_path="$WORKTREES_DIR/$name"
    
    if [ ! -d "$worktree_path" ]; then
        print_error "Worktree '$name' does not exist"
        exit 1
    fi
    
    print_status "Removing worktree: $name"
    git worktree remove "$worktree_path"
    print_success "Worktree '$name' removed successfully"
}

# Function to clean all worktrees
clean_worktrees() {
    print_warning "This will remove all worktrees. Are you sure? (y/N)"
    read -r response
    if [[ "$response" =~ ^[Yy]$ ]]; then
        print_status "Cleaning all worktrees..."
        
        # Remove all worktrees
        if [ -d "$WORKTREES_DIR" ]; then
            find "$WORKTREES_DIR" -maxdepth 1 -type d -name "*" | while read -r dir; do
                if [ "$dir" != "$WORKTREES_DIR" ]; then
                    name=$(basename "$dir")
                    if [ -f "$dir/.git" ]; then
                        print_status "Removing worktree: $name"
                        git worktree remove "$dir" 2>/dev/null || true
                    fi
                fi
            done
        fi
        
        # Remove worktrees directory
        rm -rf "$WORKTREES_DIR"
        
        # Prune stale worktrees
        git worktree prune
        
        print_success "All worktrees cleaned successfully"
    else
        print_status "Operation cancelled"
    fi
}

# Function to show status of all worktrees
show_status() {
    print_status "Status of all worktrees:"
    echo ""
    
    # Show main worktree status
    echo "Main worktree ($(pwd)):"
    git status --porcelain | head -5
    if [ $(git status --porcelain | wc -l) -gt 5 ]; then
        echo "... and $(( $(git status --porcelain | wc -l) - 5 )) more files"
    fi
    echo ""
    
    # Show feature worktree status
    if [ -d "$WORKTREES_DIR" ]; then
        find "$WORKTREES_DIR" -maxdepth 1 -type d -name "*" | while read -r dir; do
            if [ "$dir" != "$WORKTREES_DIR" ] && [ -f "$dir/.git" ]; then
                name=$(basename "$dir")
                echo "Worktree '$name' ($dir):"
                (cd "$dir" && git status --porcelain | head -5)
                if [ $(cd "$dir" && git status --porcelain | wc -l) -gt 5 ]; then
                    echo "... and $(( $(cd "$dir" && git status --porcelain | wc -l) - 5 )) more files"
                fi
                echo ""
            fi
        done
    fi
}

# Function to sync worktree with main branch
sync_worktree() {
    if [ $# -ne 1 ]; then
        print_error "Usage: $0 sync <name>"
        exit 1
    fi
    
    name="$1"
    worktree_path="$WORKTREES_DIR/$name"
    
    if [ ! -d "$worktree_path" ]; then
        print_error "Worktree '$name' does not exist"
        exit 1
    fi
    
    print_status "Syncing worktree '$name' with main branch..."
    
    # Get current branch in worktree
    current_branch=$(cd "$worktree_path" && git branch --show-current)
    
    # Update main branch
    git checkout main
    git pull origin main
    
    # Merge main into feature branch
    git checkout "$current_branch"
    git merge main
    
    print_success "Worktree '$name' synced successfully"
}

# Function to test worktree isolation
test_worktrees() {
    print_status "Testing worktree isolation..."
    
    # Create test files in each worktree
    echo "Main worktree test" > main_test.txt
    echo "Rendering optimization test" > "$WORKTREES_DIR/rendering-optimization/rendering_test.txt"
    echo "UI enhancements test" > "$WORKTREES_DIR/ui-enhancements/ui_test.txt"
    echo "Security compliance test" > "$WORKTREES_DIR/security-compliance/security_test.txt"
    
    # Check isolation
    print_status "Checking isolation..."
    
    if [ -f "rendering_test.txt" ] || [ -f "ui_test.txt" ] || [ -f "security_test.txt" ]; then
        print_error "Isolation test failed: files from other worktrees found in main"
        return 1
    fi
    
    if [ ! -f "$WORKTREES_DIR/rendering-optimization/rendering_test.txt" ]; then
        print_error "Isolation test failed: rendering test file not found"
        return 1
    fi
    
    if [ ! -f "$WORKTREES_DIR/ui-enhancements/ui_test.txt" ]; then
        print_error "Isolation test failed: UI test file not found"
        return 1
    fi
    
    if [ ! -f "$WORKTREES_DIR/security-compliance/security_test.txt" ]; then
        print_error "Isolation test failed: security test file not found"
        return 1
    fi
    
    # Clean up test files
    rm -f main_test.txt
    rm -f "$WORKTREES_DIR/rendering-optimization/rendering_test.txt"
    rm -f "$WORKTREES_DIR/ui-enhancements/ui_test.txt"
    rm -f "$WORKTREES_DIR/security-compliance/security_test.txt"
    
    print_success "Worktree isolation test passed"
}

# Main script logic
case "${1:-help}" in
    setup)
        setup_worktrees
        ;;
    list)
        list_worktrees
        ;;
    add)
        add_worktree "$2" "$3"
        ;;
    remove)
        remove_worktree "$2"
        ;;
    clean)
        clean_worktrees
        ;;
    status)
        show_status
        ;;
    sync)
        sync_worktree "$2"
        ;;
    test)
        test_worktrees
        ;;
    help|--help|-h)
        show_usage
        ;;
    *)
        print_error "Unknown command: $1"
        show_usage
        exit 1
        ;;
esac
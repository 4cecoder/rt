# Git Worktree Setup and Usage Guide

## Overview

This document describes the git worktree setup for the RT terminal emulator project, enabling parallel development streams for different feature areas.

## Worktree Structure

The project uses git worktrees to maintain separate working directories for different development streams:

```
/root/bytecats/tools/rt/                    # Main worktree (main branch)
├── worktrees/
│   ├── rendering-optimization/             # feature/rendering-optimization branch
│   ├── ui-enhancements/                    # feature/ui-enhancements branch
│   └── security-compliance/               # feature/security-compliance branch
```

## Worktree Configuration

### Main Worktree
- **Location**: `/root/bytecats/tools/rt/`
- **Branch**: `main`
- **Purpose**: Primary development branch, stable releases

### Feature Worktrees

#### 1. Rendering Optimization Worktree
- **Location**: `/root/bytecats/tools/rt/worktrees/rendering-optimization/`
- **Branch**: `feature/rendering-optimization`
- **Purpose**: Development of rendering performance optimizations
- **Focus Areas**:
  - Terminal rendering engine improvements
  - Performance benchmarking
  - Memory optimization
  - GPU acceleration (if applicable)

#### 2. UI Enhancements Worktree
- **Location**: `/root/bytecats/tools/rt/worktrees/ui-enhancements/`
- **Branch**: `feature/ui-enhancements`
- **Purpose**: Development of user interface improvements
- **Focus Areas**:
  - Theme system
  - Customization options
  - User experience improvements
  - Accessibility features

#### 3. Security Compliance Worktree
- **Location**: `/root/bytecats/tools/rt/worktrees/security-compliance/`
- **Branch**: `feature/security-compliance`
- **Purpose**: Development of security and compliance features
- **Focus Areas**:
  - Security hardening
  - Compliance validation
  - Audit logging
  - Secure configuration management

## Usage Guidelines

### Switching Between Worktrees

```bash
# Navigate to a specific worktree
cd worktrees/rendering-optimization
cd worktrees/ui-enhancements
cd worktrees/security-compliance

# Return to main worktree
cd ../..
```

### Managing Worktrees

#### List All Worktrees
```bash
git worktree list
```

#### Add a New Worktree
```bash
git worktree add <path> <branch>
```

#### Remove a Worktree
```bash
git worktree remove <path>
```

#### Prune Stale Worktrees
```bash
git worktree prune
```

### Development Workflow

#### 1. Feature Development
1. Navigate to the appropriate worktree
2. Make changes and test independently
3. Commit changes within the worktree
4. Changes are isolated from other worktrees

#### 2. Integration
1. When a feature is ready, merge the feature branch into main:
   ```bash
   cd ../..  # Return to main worktree
   git merge feature/rendering-optimization
   ```

#### 3. Conflict Resolution
- Conflicts are resolved in the main worktree during merge
- Each worktree maintains its own working directory state
- Use standard git conflict resolution procedures

### Best Practices

#### Worktree Management
- Keep worktrees clean and focused on their specific feature area
- Regularly sync with the main branch to avoid large divergences
- Remove unused worktrees to save disk space

#### Branch Management
- Use descriptive branch names
- Keep feature branches focused and small
- Regularly merge main branch changes into feature branches

#### Commit Practices
- Follow the commit message format specified in AGENTS.md
- Make atomic commits that address single concerns
- Test changes thoroughly before committing

### Isolation and Shared History

#### Isolation
- Each worktree has its own working directory
- Changes in one worktree do not affect others
- Separate git status for each worktree

#### Shared History
- All worktrees share the same git repository history
- Commits made in one worktree are visible in all others
- Branches can be merged across worktrees

### Testing the Setup

#### Verification Commands
```bash
# Check worktree structure
git worktree list

# Verify isolation
echo "Test change" >> worktrees/rendering-optimization/test.txt
git status  # Should show no changes in main worktree

# Verify shared history
cd worktrees/rendering-optimization
git commit -am "Test commit"
cd ../..
git log --oneline feature/rendering-optimization  # Should show the new commit
```

#### Expected Behavior
- Changes in one worktree don't appear in others
- Commits are shared across all worktrees
- Each worktree maintains its own staged/unstaged changes

## Troubleshooting

### Common Issues

#### Worktree Already Exists
```bash
# Error: fatal: 'worktrees/rendering-optimization' already exists
# Solution: Remove the existing worktree directory and prune
rm -rf worktrees/rendering-optimization
git worktree prune
git worktree add worktrees/rendering-optimization feature/rendering-optimization
```

#### Branch Already Checked Out
```bash
# Error: fatal: 'feature/rendering-optimization' is already checked out at 'worktrees/rendering-optimization'
# Solution: Use the existing worktree or remove it first
```

#### Detached HEAD State
```bash
# If you find yourself in detached HEAD state
git checkout <branch-name>
```

### Maintenance Commands

#### Clean Up Worktrees
```bash
# Remove all worktrees
git worktree list | grep -v main | awk '{print $1}' | xargs -I {} git worktree remove {}
git worktree prune

# Remove worktrees directory
rm -rf worktrees/
```

#### Recreate Worktrees
```bash
# Recreate all worktrees from scratch
mkdir -p worktrees
git worktree add worktrees/rendering-optimization feature/rendering-optimization
git worktree add worktrees/ui-enhancements feature/ui-enhancements
git worktree add worktrees/security-compliance feature/security-compliance
```

## Integration with CI/CD

### Build Configuration
- Each worktree can be built independently
- Use the same build commands as specified in AGENTS.md
- Test isolation by building different worktrees simultaneously

### Testing Strategy
- Run tests in each worktree to verify feature isolation
- Integration tests should be run in the main worktree after merging
- Use worktree-specific test configurations if needed

## Performance Considerations

### Disk Space
- Each worktree creates a full working directory
- Consider disk space requirements when adding many worktrees
- Remove unused worktrees to free up space

### Memory Usage
- Multiple worktrees can be worked on simultaneously
- Each worktree maintains its own git state
- Monitor memory usage when working with large repositories

## Conclusion

The git worktree setup enables efficient parallel development of different feature areas while maintaining proper isolation and shared history. Follow the guidelines and best practices outlined in this document to ensure smooth development workflows.

For questions or issues with the worktree setup, refer to the troubleshooting section or consult the git worktree documentation.
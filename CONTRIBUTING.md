# Contributing to kilar 🤝

Thank you for your interest in contributing to kilar! This document outlines the process for contributing to this project.

## Table of Contents

- [Getting Started](#getting-started)
- [Development Environment](#development-environment)
- [Code Style](#code-style)
- [Testing](#testing)
- [Pull Request Process](#pull-request-process)
- [Issue Guidelines](#issue-guidelines)
- [Code of Conduct](#code-of-conduct)

## Getting Started

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/kilar.git
   cd kilar
   ```
3. Add the original repository as an upstream remote:
   ```bash
   git remote add upstream https://github.com/polidog/kilar.git
   ```

## Development Environment

### Prerequisites

- Rust 1.70.0 or higher
- Cargo (comes with Rust)
- Git

### Setup

1. Install Rust from [rustup.rs](https://rustup.rs/)
2. Clone the repository
3. Build the project:
   ```bash
   cargo build
   ```
4. Run tests to ensure everything is working:
   ```bash
   cargo test
   ```

### Development Dependencies

The project uses several system commands:
- **macOS/Linux**: `lsof` (usually pre-installed)
- **Windows**: `netstat` (pre-installed)

## Code Style

We follow standard Rust conventions and use the following tools:

### Formatting
- Use `cargo fmt` to format your code
- We use the default rustfmt configuration

### Linting
- Use `cargo clippy` to catch common mistakes
- All clippy warnings should be addressed before submitting

### Documentation
- All public APIs should have rustdoc comments
- Include examples in documentation where appropriate
- Use `cargo doc --open` to build and view documentation

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

### Test Structure

- **Unit tests**: Located in the same file as the code (`#[cfg(test)]` modules)
- **Integration tests**: Located in the `tests/` directory
- **Documentation tests**: Embedded in rustdoc comments

### Test Guidelines

1. Write tests for all public APIs
2. Include both positive and negative test cases
3. Use descriptive test names
4. Mock external dependencies when possible
5. Ensure tests are deterministic and don't depend on system state

## Pull Request Process

1. **Create a feature branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes**:
   - Follow the code style guidelines
   - Add tests for new functionality
   - Update documentation as needed

3. **Test your changes**:
   ```bash
   cargo test
   cargo clippy
   cargo fmt --check
   ```

4. **Commit your changes**:
   - Use clear, descriptive commit messages
   - Follow conventional commit format if possible:
     ```
     feat: add new feature
     fix: resolve bug in port detection
     docs: update README
     test: add integration tests
     ```

5. **Push to your fork**:
   ```bash
   git push origin feature/your-feature-name
   ```

6. **Create a Pull Request**:
   - Provide a clear description of the changes
   - Reference any related issues
   - Include screenshots if relevant
   - Ensure all CI checks pass

### PR Requirements

- [ ] All tests pass
- [ ] Code is properly formatted (`cargo fmt`)
- [ ] No clippy warnings
- [ ] Documentation is updated
- [ ] CHANGELOG.md is updated (if applicable)

## Issue Guidelines

### Bug Reports

When reporting bugs, please include:

1. **Environment information**:
   - Operating system and version
   - Rust version (`rustc --version`)
   - kilar version

2. **Steps to reproduce**:
   - Exact commands run
   - Expected behavior
   - Actual behavior

3. **Additional context**:
   - Error messages
   - Relevant system information
   - Screenshots if applicable

### Feature Requests

When requesting features:

1. Describe the problem you're trying to solve
2. Explain why this feature would be useful
3. Provide examples of how it would be used
4. Consider alternative solutions

### Security Issues

For security-related issues, please email directly instead of creating a public issue.

## Development Guidelines

### Adding New Commands

1. Create a new module in `src/commands/`
2. Implement the command structure
3. Add CLI integration in `src/cli.rs`
4. Add validation in `src/utils/validation.rs`
5. Write comprehensive tests
6. Update documentation

### Error Handling

- Use the custom `Error` enum defined in `src/error.rs`
- Provide helpful error messages with context
- Include suggestions for resolution when possible
- Test error conditions thoroughly

### Cross-Platform Considerations

- Test on multiple platforms when possible
- Use appropriate system commands for each platform
- Handle platform-specific edge cases
- Document platform limitations

## Code of Conduct

### Our Standards

- Be respectful and inclusive
- Welcome newcomers and help them learn
- Focus on constructive feedback
- Assume good intentions

### Unacceptable Behavior

- Harassment or discrimination of any kind
- Trolling, insulting, or derogatory comments
- Personal or political attacks
- Publishing private information without consent

### Enforcement

Violations of the code of conduct should be reported to the project maintainers. All reports will be handled confidentially.

## Getting Help

- **Questions**: Open a GitHub issue with the "question" label
- **Discussions**: Use GitHub Discussions for broader topics
- **Real-time chat**: [If applicable, add Discord/Slack links]

## Recognition

Contributors will be recognized in:
- The project's README
- Release notes for significant contributions
- Special thanks in documentation

---

Thank you for contributing to kilar! 🎉
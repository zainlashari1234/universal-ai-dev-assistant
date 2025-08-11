# Contributing to Universal AI Development Assistant

Thank you for your interest in contributing to UAIDA! This document provides guidelines and information for contributors.

## 🚀 Quick Start

1. **Fork the repository**
2. **Clone your fork**
   ```bash
   git clone https://github.com/your-username/universal-ai-dev-assistant.git
   cd universal-ai-dev-assistant
   ```
3. **Set up development environment**
   ```bash
   make setup
   ```
4. **Start development**
   ```bash
   make dev
   ```

## 📋 Development Setup

### Prerequisites

- **Rust 1.70+** - Backend development
- **Node.js 18+** - Frontend and extension development
- **Docker** - Containerization (optional)
- **VS Code** - Recommended IDE

### Environment Setup

```bash
# Install all dependencies
make install

# Run tests
make test

# Start development servers
make dev

# Build everything
make build
```

## 🏗️ Project Structure

```
universal-ai-dev-assistant/
├── backend/           # Rust backend with AI engine
├── frontend/          # React web interface
├── extensions/        # IDE extensions
│   ├── vscode/       # VS Code extension
│   ├── jetbrains/    # JetBrains plugin (planned)
│   └── vim/          # Vim plugin (planned)
├── cli/              # Command-line interface
├── docs/             # Documentation
└── examples/         # Example projects
```

## 🤝 How to Contribute

### 1. Issues

- **Bug Reports**: Use the bug report template
- **Feature Requests**: Use the feature request template
- **Questions**: Use GitHub Discussions

### 2. Pull Requests

1. **Create a branch** from `develop`
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes**
   - Follow coding standards
   - Add tests for new features
   - Update documentation

3. **Test your changes**
   ```bash
   make test
   make lint
   ```

4. **Commit with conventional commits**
   ```bash
   git commit -m "feat: add new completion algorithm"
   git commit -m "fix: resolve memory leak in AI engine"
   git commit -m "docs: update API documentation"
   ```

5. **Push and create PR**
   ```bash
   git push origin feature/your-feature-name
   ```

### 3. Coding Standards

#### Rust (Backend)
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Follow Rust naming conventions
- Add documentation comments (`///`)
- Write unit tests

#### TypeScript (Frontend/Extension)
- Use Prettier for formatting
- Use ESLint for linting
- Follow React best practices
- Use TypeScript strictly
- Write component tests

#### General
- Use meaningful commit messages
- Keep PRs focused and small
- Update documentation
- Add tests for new features

## 🧪 Testing

### Backend Tests
```bash
cd backend
cargo test
```

### Frontend Tests
```bash
cd frontend
npm test
```

### Extension Tests
```bash
cd extensions/vscode
npm test
```

### Integration Tests
```bash
make test
```

## 📚 Documentation

- **API Documentation**: Generated from code comments
- **User Guide**: Located in `docs/`
- **Developer Guide**: This file and `docs/development.md`

### Writing Documentation

- Use clear, concise language
- Include code examples
- Update relevant sections when making changes
- Use Markdown formatting

## 🎯 Areas for Contribution

### High Priority
- **AI Model Integration**: Improve local LLM support
- **Language Support**: Add new programming languages
- **Performance**: Optimize response times
- **Security**: Enhance privacy and security features

### Medium Priority
- **UI/UX**: Improve user interface
- **Documentation**: Expand guides and examples
- **Testing**: Increase test coverage
- **DevOps**: Improve CI/CD pipeline

### Good First Issues
- **Bug fixes**: Small, well-defined bugs
- **Documentation**: Fix typos, add examples
- **Tests**: Add missing test cases
- **Refactoring**: Code cleanup and organization

## 🏷️ Labels

- `good first issue` - Good for newcomers
- `help wanted` - Extra attention needed
- `bug` - Something isn't working
- `enhancement` - New feature or request
- `documentation` - Improvements to docs
- `performance` - Performance improvements
- `security` - Security-related issues

## 🔄 Release Process

1. **Development** happens on `develop` branch
2. **Features** are merged via PRs
3. **Releases** are created from `main` branch
4. **Versioning** follows Semantic Versioning

### Version Bumping
- **Patch** (0.0.x): Bug fixes
- **Minor** (0.x.0): New features, backward compatible
- **Major** (x.0.0): Breaking changes

## 🌟 Recognition

Contributors are recognized in:
- **README.md** contributors section
- **Release notes** for significant contributions
- **GitHub contributors** page

## 📞 Getting Help

- **GitHub Discussions**: General questions
- **GitHub Issues**: Bug reports and feature requests
- **Discord**: Real-time chat (link in README)
- **Email**: maintainers@uaida.dev

## 📜 Code of Conduct

We follow the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). Please read it before contributing.

## 🎉 Thank You!

Every contribution, no matter how small, helps make UAIDA better for everyone. We appreciate your time and effort!

---

**Happy Coding!** 🚀
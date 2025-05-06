# 🧙‍♂️ Contributing to Aurorium

Thank you for your interest in contributing to **Aurorium** — the file manager for the Revive101 project!  
We appreciate every idea, pull request, and bug report. This document will help you get started.

---

## 🛠️ What Can I Work On?

You can contribute in many ways:

- 🐛 **Report bugs** via [GitHub Issues](https://github.com/Revive101/Aurorium/issues)
- ✨ **Suggest new features** or improvements
- 🧪 **Test** Aurorium on different systems and configurations
- 🧹 **Refactor** or clean up existing code
- 📚 **Improve documentation** (README, code comments, etc.)
- 🔌 **Add new tools**, extensions, or CLI features

---

## 🚀 Getting Started

1. **Fork** the repository to your own GitHub account.
2. **Clone** your fork locally:

   ```bash
   git clone https://github.com/<your-username>/Aurorium.git
   cd Aurorium
   ```

3. Create a new branch:

   ```bash
   git checkout -b your-feature-name
   ```

4. Make your changes! Try to keep commits clean and atomic.
5. Run tests, linting, or check formatting before submitting.
6. **Push** your branch and open a **Pull Request** to the main repository.

---

## 🧪 Testing & Running Locally

To run the project locally:

```bash
cargo run
```

To build:

```bash
cargo build --release
```

Ensure that you are using the correct Rust toolchain for your platform. See [README.md](./README.md#getting-started) for more.

---

## 💬 Commit Message Style

Try to keep your commits readable and meaningful:

```
feat: add support for revision auto-detection
fix: resolve panic when no endpoint is provided
docs: update usage instructions in README
refactor: simplify downloader task queue
```

---

## 🧹 Code Style

We use idiomatic Rust conventions and `rustfmt`. Before submitting:

```bash
cargo fmt
```

Your code must pass all tests:

```bash
cargo test
```

Also, check for clippy warnings:

```bash
cargo clippy
```

---

## 🙌 Contributor Recognition

All accepted contributions are rewarded with the `@Contributor` role in our [Discord server](https://discord.gg/sMFgyNRDDM)!  
Make sure your **GitHub** is linked to **Discord** so we can recognize you.

---

## 📜 Code of Conduct

Please be respectful in all communications. Harassment, discrimination, or disrespectful behavior will not be tolerated.  
By participating, you agree to follow our community standards.

---

## 🔗 Need Help?

If you’re stuck or have a question:

- Check [open issues](https://github.com/Revive101/Aurorium/issues)
- Ask in our [Discord](https://discord.gg/sMFgyNRDDM) in the appropriate channel

---

We’re excited to build Aurorium with you 💫

<p align="center" style="text-align: center">
  <img src="https://raw.githubusercontent.com/Tarikul-Islam-Anik/Animated-Fluent-Emojis/master/Emojis/Travel%20and%20places/Ringed%20Planet.png" style="display: block; margin: 0 auto;">
  <h1 style="text-align: center; margin-top: 0;" align="center">Aurorium</h1>
  <p align="center">
    <img src="https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white" alt="Rust">
    <img src="https://img.shields.io/badge/license-CC%20BY--NC--SA%204.0-lightgrey.svg?style=for-the-badge" alt="License">
    <a href="https://discord.gg/sMFgyNRDDM"><img src="https://img.shields.io/discord/940647911182729257?color=5865F2&label=Discord&logo=discord&logoColor=white&style=for-the-badge" alt="Discord"></a>
  </p>
</p>

---

> [!IMPORTANT]  
> **Disclaimer:** We are not affiliated with Wizard101Rewritten in any way and do not tolerate any use of this project in reference to Wizard101Rewritten!

---

## 📌 Table of Contents

- [❓ Introduction](#-introduction)
- [🚀 Getting Started](#-getting-started)
- [🧰 Parameters](#-parameters)
- [🔧 Contributing](#-contributing)
- [🌐 Community](#-community)
- [📝 License](#-license)

---

## ❓ Introduction

Aurorium is the backbone of the Revive101 project, providing essential file management for the Wizard101 client revival. Our goal is to create an open, collaborative environment where the community can contribute to bringing back the magic of Wizard101.

---

## 🚀 Getting Started

These steps help you set up **Aurorium** for development and testing.

> [!NOTE]
> If you just want to use the executable, **you can skip to the [releases page](https://github.com/Revive101/Aurorium/releases/latest)** and download the latest version directly.

### ✅ Prerequisites

- [Rust](https://www.rust-lang.org/)
- A code editor like [VS Code](https://code.visualstudio.com/)
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) extension

If you're on Linux you'll need to install following packages:

```bash
sudo apt install build-essential pkg-config libssl-dev
```

### 📥 Installation

Clone the repository:

```bash
git clone https://github.com/Revive101/Aurorium.git
cd Aurorium
```

### ▶️ Running

To run in debug mode:

```bash
cargo run
```

To build:

```bash
cargo build         # Debug build
cargo build --release  # Optimized release build
```

From **version 2.0**, Aurorium automatically fetches the latest revision from the server.  
For **versions < 2.0**, use the `--revision` or `-r` parameter manually.

---

### ⚠️ Common Errors

**`link.exe not found` (Windows):**  
Install Microsoft C++ Build Tools, or run:

```bash
rustup toolchain install stable-x86_64-pc-windows-msvc
rustup default stable-x86_64-pc-windows-msvc
```

**On Linux:**  
Use target `x86_64-unknown-linux-gnu`.

---

### 📁 Revision Format

Revisions follow the format:

```
V_r[Major].[Minor]
```

Examples:

- `V_r746756.WizardDev`
- `V_r766982.Wizard_1_560_0_Live`

---

## 🧰 Parameters

The compiled executable supports the following CLI arguments:

```bash
  -e, --endpoint <ENDPOINT>                          [env: ENDPOINT=] [default: 127.0.0.1:12369]
  -c, --concurrent-downloads <CONCURRENT_DOWNLOADS>  [env: CONCURRENT_DOWNLOADS=] [default: 2]
  -s, --save-directory <SAVE_DIRECTORY>              [env: SAVE_DIRECTORY=] [default: data]
      --host <HOST>                                  [env: HOST=] [default: patch.us.wizard101.com]
      --port <PORT>                                  [env: PORT=] [default: 12500]
  -f, --fetch-interval <FETCH_INTERVAL>              [default: 28800]
  -m, --max-requests <MAX_REQUESTS>                  [default: 256]
  -r, --reset-interval <RESET_INTERVAL>              [default: 60]
  -t, --timeout <TIMEOUT>                            [default: 10]
```

---

## 🔧 Contributing

We welcome all contributions! Whether you’re a Rust wizard or a curious apprentice, your input helps us grow.

- 📜 Read our [Contributing Guidelines](./CONTRIBUTING.md).
- 🍴 Fork the repo, make your changes, and submit a pull request.
- 🐛 Report bugs or suggest features via [issues](https://github.com/Revive101/Aurorium/issues).

> [!NOTE]
> Contributors can request the `@Contributor` role in our [Discord](https://discord.gg/sMFgyNRDDM).  
> Make sure your GitHub is linked to your Discord account.

---

## 🌐 Community

Join us on [Discord](https://discord.gg/sMFgyNRDDM) to meet other fans, developers, and contributors!

---

## 📝 License

<p xmlns:cc="http://creativecommons.org/ns#" xmlns:dct="http://purl.org/dc/terms/"><a property="dct:title" rel="cc:attributionURL" href="https://github.com/Revive101/Aurorium">Aurorium</a> by <a rel="cc:attributionURL dct:creator" property="cc:attributionName" href="https://github.com/Phill030/">Phill030</a> is licensed under <a href="http://creativecommons.org/licenses/by-nc-sa/4.0/?ref=chooser-v1" target="_blank" rel="license noopener noreferrer" style="display:inline-block;">CC BY-NC-SA 4.0<img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/cc.svg?ref=chooser-v1"><img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/by.svg?ref=chooser-v1"><img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/nc.svg?ref=chooser-v1"><img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/sa.svg?ref=chooser-v1"></a></p>

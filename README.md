<h1 align="center">Aurorium</h1>
<p align="center">Aurorium is the heart (*Not the core!*) of the Revive101 project, designed to manage the files associated with the Wizard101 client revival. We've open-sourced Aurorium to foster transparency, collaboration, and community involvement in our mission to bring back the magic of Wizard101.</p>
<h4 align="center"><b>Disclaimer:</b> we are not affiliated with Wizard101Rewritten in any way! <a href="https://discord.gg/sMFgyNRDDM">Discord invite</a></h4>

-----------------

- [Getting Started](#getting-started)
- [Contributing](#contributing)
- [Community](#community)
- [License](#license)

## Getting Started

These instructions will help you get a copy of Aurorium up and running on your local machine for development and testing purposes.

### Prerequisites

- [Rust programming language](https://www.rust-lang.org/).
- [VSCode](https://code.visualstudio.com/) (Alternatively [VSCodium](https://vscodium.com/))
- [rust-analyzer extension](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

### Installation

Install [`git`](https://git-scm.com/) to clone this repository to your local machine. Run following command in your designated folder:

```bash
git clone https://github.com/Revive101/Aurorium.git
```

### Usage

To run the executable, use `cargo run` alternatively you can build it using `cargo build` or in release `cargo build --release`.

### Parameters

You can provide the (built) executable with following parameters:

```
    -v, --verbose               Activate verbosity (Default: warn)
    -r, --revision=<String>     Fetch from a revision string (Example V_r740872.Wizard_1_520)
    -i, --ip=<SocketAddr>       Override the default endpoint IP (Default: 0.0.0.0:12369)
    -c, --concurrent_downloads=<usize>  Override the count of concurrent downloads at once (Default: 8)
        --max_requests=<u32>    Change the amount of requests a user can send before getting rate-limited by the server
        --reset_duration=<u32>  Change the duration for the interval in which the rate-limit list get's cleared (In seconds)
    -h, --help                  Prints help information
```

## Contributing

We welcome contributions from the community! Whether you're an experienced developer or just getting started, there are many ways to contribute to Aurorium's development:

Check out our [Contributing Guidelines](TODO) for detailed information on how to contribute.
Fork the repository, make your changes, and submit a pull request.
Report bugs or suggest new features by opening [issues](https://github.com/Revive101/Aurorium/issues).

## Community

Join the Revive101 community on [discord](https://discord.gg/sMFgyNRDDM) to connect with fellow Wizards, developers, and enthusiasts.

## License

Aurorium is licensed under the [GNU General Public License v3.0](LICENSE.md), which means you are free to use, modify, and distribute the code as long as you comply with the terms and conditions of the GPL-3.0 license. This license ensures that the software remains open source and that any derivative works are also subject to the same open source terms.

For more details, please refer to the [GNU General Public License v3.0](LICENSE.md) included in this repository.

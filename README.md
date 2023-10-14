<h1 align="center">Aurorium</h1>
<p align="center">Aurorium is the heart (*Not the core!*) of the Revive101 project, designed to manage the files associated with the Wizard101 client revival. We've open-sourced Aurorium to foster transparency, collaboration, and community involvement in our mission to bring back the magic of Wizard101.</p>
<h4 align="center"><b>Disclaimer:</b> we are not affiliated with Wizard101Rewritten in any way! <a href="https://discord.gg/sMFgyNRDDM">Discord invite</a></h4>

---

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
-r, --revision              Fetch from a revision string (Example V_r740872.Wizard_1_520)
-i, --ip=<SocketAddr>       Override the default endpoint IP (Default: 0.0.0.0:12369)
-c, --concurrent_downloads  Override the count of concurrent downloads at once (Default: 8)
    --max_requests          Change the amount of requests a user can send before getting rate-limited by the server
    --reset_duration        Change the duration for the interval in which the rate-limit list get's cleared (In seconds)
-h, --help                  Prints this message
```

## Contributing

We welcome contributions from the community! Whether you're an experienced developer or just getting started, there are many ways to contribute to Aurorium's development:

Check out our [Contributing Guidelines](TODO) for detailed information on how to contribute.
Fork the repository, make your changes, and submit a pull request.
Report bugs or suggest new features by opening [issues](https://github.com/Revive101/Aurorium/issues).

## Community

Join the Revive101 community on [discord](https://discord.gg/sMFgyNRDDM) to connect with fellow Wizards, developers, and enthusiasts.

## License

<p xmlns:cc="http://creativecommons.org/ns#" xmlns:dct="http://purl.org/dc/terms/"><a property="dct:title" rel="cc:attributionURL" href="https://github.com/Phill030/KiWad-Unpacker">KiWad-Unpacker</a> by <a rel="cc:attributionURL dct:creator" property="cc:attributionName" href="https://github.com/Phill030/">Phill030</a> is licensed under <a href="http://creativecommons.org/licenses/by-nc-sa/4.0/?ref=chooser-v1" target="_blank" rel="license noopener noreferrer" style="display:inline-block;">CC BY-NC-SA 4.0<img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/cc.svg?ref=chooser-v1"><img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/by.svg?ref=chooser-v1"><img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/nc.svg?ref=chooser-v1"><img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/sa.svg?ref=chooser-v1"></a></p>

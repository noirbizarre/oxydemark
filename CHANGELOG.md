# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1](https://github.com/noirbizarre/oxydemark/compare/0.2.0..0.2.1) - 2026-08-03

### 🐛 Bug Fixes

- **deps** Upgrade to rushdown 0.18 and pin the companion crates - ([623e49a](https://github.com/noirbizarre/oxydemark/commit/623e49ab37388a97eeef441a3c403265e1f1d358))
- **render** Align both render paths on void elements and raw HTML - ([811e571](https://github.com/noirbizarre/oxydemark/commit/811e571813e80dd541a2cf2976f1ad5ab21965ac))

### 📚 Documentation

- Record the rushdown 0.18 pinning policy - ([e99587b](https://github.com/noirbizarre/oxydemark/commit/e99587bd4f3d490e93985b543b6d6c0efd59c480))
- Record the coverage setup in OMEP-0012 - ([f14da26](https://github.com/noirbizarre/oxydemark/commit/f14da26deac77f11c69514a0de29062298b7619d))

### 🔧 CI

- Guard against rushdown companion-crate drift - ([c72b38f](https://github.com/noirbizarre/oxydemark/commit/c72b38f90a45b675962b93abf2f82215045d4336))
- Report rust and python coverage to codecov - ([16f50cb](https://github.com/noirbizarre/oxydemark/commit/16f50cbfbce9170a80be1d7e544d52609ee37935))

## 0.2.0 - 2026-08-03

### 💫 Features

- **comark** Handle emojis and comark components syntax - ([1f2e1ab](https://github.com/noirbizarre/oxydemark/commit/1f2e1ab437306f21e792fe97a8a69fe495a47f71))
- **contrib** Add example admonition, shortcode, mention and lazy-image plugins - ([ef57435](https://github.com/noirbizarre/oxydemark/commit/ef5743599beabf67c8ba9b38c785d120ac8292ba))
- HTML rendering for slots and nested components - ([f75246a](https://github.com/noirbizarre/oxydemark/commit/f75246a390706caa2f34b7f42325d70ff5d82e0b))
- Table-of-contents extraction API - ([3b371e2](https://github.com/noirbizarre/oxydemark/commit/3b371e2cec70d9495eb7e9a829c66b7e6aedf861))
- Nested components via multi-colon fences - ([7b55840](https://github.com/noirbizarre/oxydemark/commit/7b558406118792b6b5f18b5f3337d58733d90f55))
- In-component YAML props - ([084d030](https://github.com/noirbizarre/oxydemark/commit/084d03060e7cfb105920081890022c1fc34415e1))
-  🚨 **breaking** Expose PyO3-independent public Rust API - ([4435315](https://github.com/noirbizarre/oxydemark/commit/4435315612cc199f0aa32554bc0daa8a4d7d37d8))
- Typed frontmatter accessor on parse result - ([74a1194](https://github.com/noirbizarre/oxydemark/commit/74a119475b22ce84491768fc81e326b195e721f0))
- Extract &lt;!-- more --&gt; summary - ([3eaa33f](https://github.com/noirbizarre/oxydemark/commit/3eaa33f62c1f07c4042f99e7c5764e31b42aff08))
- Heading anchors/slugs - ([470bdfb](https://github.com/noirbizarre/oxydemark/commit/470bdfb1a01bb31b4de09ebb54d996cbaf2dd587))
- Parse component slots (#slot-name) - ([06013e2](https://github.com/noirbizarre/oxydemark/commit/06013e23e86cc2ec43d0484e5f5573aeed041faa))
- Initial implementation - ([183febc](https://github.com/noirbizarre/oxydemark/commit/183febcec1caa86451de0496539ac46ce99fa737))
- Initial import - ([f143747](https://github.com/noirbizarre/oxydemark/commit/f14374762c0786d8057f3f805b38bef60c58375d))

### 🔨 Refactor

- **lib** Split lib in multiple modules - ([c24283b](https://github.com/noirbizarre/oxydemark/commit/c24283b63e745c8d40b3835ec878bb5282521e88))
- Fix cross-codebase consistency issues - ([f16835e](https://github.com/noirbizarre/oxydemark/commit/f16835eee96ef6902c02956b8a92b873549f0b9a))

### 📚 Documentation

- **omep-0007** Finalize and accept Comark Phase 3 spec - ([5f69cdc](https://github.com/noirbizarre/oxydemark/commit/5f69cdc511e4772c58c05ed7c4734f0a2bf369db))
- **specs** Drop the `v` prefix from release tags - ([dfea36a](https://github.com/noirbizarre/oxydemark/commit/dfea36a0a625d03a6844bdd3499180ec09762ca6))
- **specs** Add OMEP-0011 documentation tooling - ([9150db7](https://github.com/noirbizarre/oxydemark/commit/9150db7c6fa5e78b777c77cfe3ea73b3c6e55609))
- **specs** Define oxydemark.contrib as a provisional public surface - ([239a764](https://github.com/noirbizarre/oxydemark/commit/239a7642f81848b5e544b2b04419efbb36dbd19d))
- Record the Release-PR model in OMEP-0009 - ([90a6bcf](https://github.com/noirbizarre/oxydemark/commit/90a6bcfb90fe40eeb5ea81eb75111cbef88f0a88))
- Document the comark compliance fixture format - ([b0cbf06](https://github.com/noirbizarre/oxydemark/commit/b0cbf06e2f844b32e75b95e658b592e08085febd))
- Convert Python docstrings to Google style - ([8f82a43](https://github.com/noirbizarre/oxydemark/commit/8f82a433cfa547bc35b5a5b0ee816947714a51ed))
- Add plugin authoring guide - ([bdfa706](https://github.com/noirbizarre/oxydemark/commit/bdfa706233153a1704c936590c7b69b22aaa2b5f))
- Add OMEP-0010 for structured metadata extraction - ([d5e646d](https://github.com/noirbizarre/oxydemark/commit/d5e646d176e41943afdf87ebceaf62d3ba4e5099))
- Add OMEP-0009 publishing & distribution spec - ([8cc13da](https://github.com/noirbizarre/oxydemark/commit/8cc13dad354a0d4d03e2f28466069c78986b7af4))
- Add OMEP-0008 public API & versioning policy - ([24d2281](https://github.com/noirbizarre/oxydemark/commit/24d228181c67bfccb8a791e3ad845ec6bb55eb58))

### 🧪 Tests

- **compliance** Add the python compliance harness - ([e8abd4a](https://github.com/noirbizarre/oxydemark/commit/e8abd4a4263cdcd99290dcec78bed7982547b5eb))
- **compliance** Add the rust compliance harness - ([3d6a8d5](https://github.com/noirbizarre/oxydemark/commit/3d6a8d5fc773f0b7d036e6fb850eac5b8e6ce661))
- **compliance** Add the comark compliance fixtures - ([ff1bcb0](https://github.com/noirbizarre/oxydemark/commit/ff1bcb0b52713dea3fcd7c1e0b4dd82d386999b4))
- **contrib** Cover the example plugins - ([c8796d7](https://github.com/noirbizarre/oxydemark/commit/c8796d7b8bbfbfb301eaad3db2a21a2df0531b01))
- **pytest** Force pytest 9+ - ([2e69c9d](https://github.com/noirbizarre/oxydemark/commit/2e69c9d79f55eaa9bfdc76f833d1aa9724f28747))
- Guard the generated API reference - ([5d4ba79](https://github.com/noirbizarre/oxydemark/commit/5d4ba79e8cb3d1b8fa14c37c840e7017dbb5517e))
- Enforce typed public API surface with ty gate - ([fe84335](https://github.com/noirbizarre/oxydemark/commit/fe8433574a06dd56da7175ec217dc61ca017ca4f))

### 🏗️ Build

- **docs** Add the zensical site and mkdocstrings API reference - ([35963f4](https://github.com/noirbizarre/oxydemark/commit/35963f47e4298558e427995887d0888d921ed039))
- **mise** Add tasks building the docs site and rustdoc - ([4abbe08](https://github.com/noirbizarre/oxydemark/commit/4abbe088dae4bebf99535c03a454cc0a0b7798ff))
- Adopt gh-ship's git-cliff and typos configuration - ([dba12cd](https://github.com/noirbizarre/oxydemark/commit/dba12cd3fc47a356670c7bb1bf78ee14b0f019b7))
- Add serde_json as a test-only dev-dependency - ([6a9bfcc](https://github.com/noirbizarre/oxydemark/commit/6a9bfccb3e7fc58e92786e9f777eb64848aa67e4))
- Build the wheel against the stable ABI - ([f1b6714](https://github.com/noirbizarre/oxydemark/commit/f1b67140eec676a453793b3b898dc35d7dd143a1))
- Declare the crate homepage - ([4f77be8](https://github.com/noirbizarre/oxydemark/commit/4f77be8a38a4ec5e33adeec74974fd0b65f8e282))
- Complete the crate metadata for publication - ([2182f82](https://github.com/noirbizarre/oxydemark/commit/2182f8223a9b6dfb764ce39f26195bf3f359f0f4))

### 🔧 CI

- Serialise the publishes so a failure cannot half-release - ([306e93b](https://github.com/noirbizarre/oxydemark/commit/306e93b8222e80ec6682221061df52b741a5e0fa))
- Release through gh-ship Release PRs - ([b36d5a4](https://github.com/noirbizarre/oxydemark/commit/b36d5a409778ce6a64a407ad3a7540ded7a80dbd))
- Run the rust test suite with the python feature - ([c01de18](https://github.com/noirbizarre/oxydemark/commit/c01de1812c302b7f682e7b2e16d8c7490feba3df))
- Scope `-Dwarnings` to the jobs building this crate - ([5a02527](https://github.com/noirbizarre/oxydemark/commit/5a02527687d122d217c28a9cd1338d1f47e0c4ce))
- Publish wheels and an sdist to PyPI on release tags - ([3126215](https://github.com/noirbizarre/oxydemark/commit/312621531bb512e9ba89d9a02f9fc79c2c0de10e))
- Add the tag-triggered crates.io release workflow - ([8df0068](https://github.com/noirbizarre/oxydemark/commit/8df0068a4a04b018ddca581a980487275b7a2fc4))
- Build the docs on pull requests and deploy to GitHub Pages - ([2de30d8](https://github.com/noirbizarre/oxydemark/commit/2de30d85b2904c20ee8bcae2a3b766671ab526b4))
- Fix wheel smoke-test on externally-managed interpreters ([#27](https://github.com/noirbizarre/oxydemark/issues/27)) - ([5dae8fa](https://github.com/noirbizarre/oxydemark/commit/5dae8faeeb931ae2193fdb3c50eb5b4a17858c14))
- Adopt uv for build and checks, add Python 3.14 - ([4b84f99](https://github.com/noirbizarre/oxydemark/commit/4b84f997832646d59fd3b7bd64a65b1318ee4a2c))

## ❤️ New Contributors

* @noirbizarre made their first contribution in [#31](https://github.com/noirbizarre/oxydemark/pull/31)

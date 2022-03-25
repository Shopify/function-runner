# Release

1. From the `main` branch, set an environment variable to the version number with the format `v{version_number}` (e.g. `v0.1.0`). This should match the version number in `Cargo.toml`.

```sh
export SCRIPT_RUNNER_VERSION=v0.1.0
```

2. Create a new tag.

```sh
git tag $SCRIPT_RUNNER_VERSION
git push origin --tags
```

2. Create a new Github release [here](https://github.com/Shopify/script-runner/releases/new).
3. When the release is created, the Github action defined in `publish.yml` will be run. This will produce the build artifacts for script-runner, except for `arm-macos` (e.g. M1 Macs).
4. Build `script-runner` on an ARM Mac (e.g. M1 Mac)

```sh
cargo build --release --package script-runner && gzip -k -f target/release/script-runner && mv target/release/script-runner.gz script-runner-arm-macos-$SCRIPT_RUNNER_VERSION.gz
```

5. Create the shasum file

```sh
shasum -a 256 script-runner-arm-macos-$SCRIPT_RUNNER_VERSION.gz | awk '{ print $1 }' > script-runner-arm-macos-$SCRIPT_RUNNER_VERSION.gz.sha256
```

6. Attach the build and shasum to the release created in step 2.

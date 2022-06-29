# Release

1. From the `main` branch, set an environment variable to the version number with the format `v{version_number}` (e.g. `v0.1.0`). This should match the version number in `Cargo.toml`.

```sh
export FUNCTION_RUNNER_VERSION=v0.1.0
```

2. Create a new tag.

```sh
git tag $FUNCTION_RUNNER_VERSION
git push origin --tags
```

2. Create a new Github release [here](https://github.com/Shopify/function-runner/releases/new).
3. When the release is created, the Github action defined in `publish.yml` will be run. This will produce the build artifacts for function-runner, except for `arm-macos` (e.g. M1 Macs).
4. Build `function-runner` on an ARM Mac (e.g. M1 Mac)

```sh
cargo build --release --package function-runner && gzip -k -f target/release/function-runner && mv target/release/function-runner.gz function-runner-arm-macos-$FUNCTION_RUNNER_VERSION.gz
```

5. Create the shasum file

```sh
shasum -a 256 function-runner-arm-macos-$FUNCTION_RUNNER_VERSION.gz | awk '{ print $1 }' > function-runner-arm-macos-$FUNCTION_RUNNER_VERSION.gz.sha256
```

6. Attach the build and shasum to the release created in step 2.
